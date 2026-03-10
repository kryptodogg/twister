/**
 * metrics-server.mjs
 *
 * WebSocket server for streaming live Mamba metrics to React frontend
 * Bridges Rust backend (via HTTP/IPC) to React WebSocket consumers
 *
 * Usage: node metrics-server.mjs
 * Listens on ws://localhost:8080/metrics
 */

import WebSocket, { WebSocketServer } from 'ws';
import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

const PORT = process.env.PORT || 8080;
const RUST_BACKEND_URL = process.env.RUST_BACKEND_URL || 'http://localhost:3000';
const METRICS_POLL_INTERVAL = 16; // ~60 FPS (16ms)

// ─────────────────────────────────────────────────────────────────────────────
// Metrics Cache & Polling
// ─────────────────────────────────────────────────────────────────────────────

let cachedMetrics = {
  anomaly_score: 0.0,
  drive: 0.0,
  foldback: 0.0,
  asymmetry: 0.5,
  latent_activity: [0.0, 0.0, 0.0],
  waveform: Array(100).fill(0),
  frame_index: 0,
  total_samples: 0,
};

// Simulated metrics generator (for demo when backend unavailable)
const generateSimulatedMetrics = () => {
  const now = Date.now();
  const time = (now % 10000) / 1000; // 0-10 second cycle

  // Simulate attack pattern: 5 second on/off cycle
  const isAttack = Math.sin((now / 5000) * Math.PI * 2) > 0;

  return {
    anomaly_score: isAttack ? 0.7 + Math.cos(now / 1000) * 0.2 : 0.1 + Math.random() * 0.05,
    drive: isAttack ? 0.8 + Math.sin(now / 1000 * 3) * 0.1 : 0.0,
    foldback: isAttack ? 0.6 + Math.cos(now / 1000 * 2.5) * 0.15 : 0.0,
    asymmetry: 0.5 + Math.sin(now / 1000 * 1.5) * 0.3,
    latent_activity: [
      Math.max(0.1, isAttack ? 0.7 + Math.cos(now / 1000) * 0.2 : 0.1),
      Math.max(0.05, isAttack ? 0.6 + Math.cos(now / 1000 * 1.5) * 0.15 : 0.05),
      0.5 + Math.sin(now / 1000 * 2) * 0.3,
    ],
    waveform: Array.from({ length: 100 }, (_, i) => {
      const t = time + (i / 100) * 0.1;
      let val = Math.sin(t * 10);
      if (isAttack) {
        val = Math.sin(val * 3); // "Smear" effect
      }
      return Math.max(-1, Math.min(1, val));
    }),
    frame_index: Math.floor(now / 10),
    total_samples: Math.floor((now % 10000) / 10) * 224,
  };
};

// Fetch metrics from Rust backend or use simulated data
const pollMetrics = async () => {
  try {
    // Try to fetch from Rust backend
    const response = await fetch(`${RUST_BACKEND_URL}/api/metrics`, {
      timeout: 1000,
    });
    if (response.ok) {
      cachedMetrics = await response.json();
    } else {
      cachedMetrics = generateSimulatedMetrics();
    }
  } catch (err) {
    // Fallback to simulated metrics
    cachedMetrics = generateSimulatedMetrics();
  }
};

// ─────────────────────────────────────────────────────────────────────────────
// HTTP Server Setup
// ─────────────────────────────────────────────────────────────────────────────

const server = http.createServer((req, res) => {
  // Serve static files
  if (req.url === '/' || req.url === '/index.html') {
    const filePath = path.join(__dirname, 'index.html');
    if (fs.existsSync(filePath)) {
      res.writeHead(200, { 'Content-Type': 'text/html' });
      res.end(fs.readFileSync(filePath, 'utf-8'));
    } else {
      res.writeHead(200, { 'Content-Type': 'text/html' });
      res.end(generateFallbackHTML());
    }
  } else if (req.url === '/metrics.jsx') {
    res.writeHead(200, { 'Content-Type': 'text/javascript' });
    res.end(fs.readFileSync(path.join(__dirname, 'waveshaper_metrics.jsx'), 'utf-8'));
  } else {
    res.writeHead(404);
    res.end('Not found');
  }
});

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket Server Setup
// ─────────────────────────────────────────────────────────────────────────────

const wss = new WebSocketServer({ server });

wss.on('connection', (ws) => {
  console.log('✓ Client connected to metrics stream');

  // Send initial metrics
  ws.send(JSON.stringify(cachedMetrics));

  // Setup interval to send metrics to this client
  const interval = setInterval(() => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(cachedMetrics));
    } else {
      clearInterval(interval);
    }
  }, METRICS_POLL_INTERVAL);

  ws.on('close', () => {
    clearInterval(interval);
    console.log('✗ Client disconnected');
  });

  ws.on('error', (err) => {
    console.error('WebSocket error:', err.message);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Metrics Polling Loop
// ─────────────────────────────────────────────────────────────────────────────

setInterval(pollMetrics, METRICS_POLL_INTERVAL);

// ─────────────────────────────────────────────────────────────────────────────
// Server Start
// ─────────────────────────────────────────────────────────────────────────────

server.listen(PORT, () => {
  console.log(`
╔════════════════════════════════════════════════════════════════╗
║           🤖 Waveshaper Metrics Server                        ║
╚════════════════════════════════════════════════════════════════╝

✓ Server listening on http://localhost:${PORT}
✓ WebSocket stream: ws://localhost:${PORT}/metrics
✓ Using ${RUST_BACKEND_URL} as backend

Open your browser to http://localhost:${PORT} to view live metrics
  `);
});

// ─────────────────────────────────────────────────────────────────────────────
// Fallback HTML (if index.html not present)
// ─────────────────────────────────────────────────────────────────────────────

function generateFallbackHTML() {
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Waveshaper Metrics</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <script src="https://unpkg.com/react@18/umd/react.development.js"></script>
  <script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
  <script src="https://unpkg.com/recharts@2.10.0/dist/Recharts.js"></script>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { font-family: 'Monaco', 'Courier New', monospace; }
  </style>
</head>
<body>
  <div id="root"></div>
  <script>
    // Inline React component
    const React = window.React;
    const ReactDOM = window.ReactDOM;
    const { LineChart, Line, ResponsiveContainer, XAxis, YAxis, CartesianGrid, Tooltip } = window.recharts;

    const WaveshaperMetrics = ({ wsUrl = 'ws://localhost:8080/metrics' }) => {
      const [metrics, setMetrics] = React.useState({
        anomaly_score: 0.0,
        drive: 0.0,
        foldback: 0.0,
        asymmetry: 0.5,
        latent_activity: [0.0, 0.0, 0.0],
        waveform: Array(100).fill(0),
        frame_index: 0,
        total_samples: 0,
        isUnderAttack: false,
      });

      const [isConnected, setIsConnected] = React.useState(false);
      const [autoSteer, setAutoSteer] = React.useState(true);
      const wsRef = React.useRef(null);

      React.useEffect(() => {
        const connectWebSocket = () => {
          try {
            wsRef.current = new WebSocket(wsUrl);

            wsRef.current.onopen = () => {
              console.log('✓ Connected to metrics stream');
              setIsConnected(true);
            };

            wsRef.current.onmessage = (event) => {
              try {
                const data = JSON.parse(event.data);
                setMetrics((prev) => ({
                  ...prev,
                  ...data,
                  isUnderAttack: (data.anomaly_score ?? 0) > 0.5,
                }));
              } catch (err) {
                console.error('Failed to parse metrics:', err);
              }
            };

            wsRef.current.onerror = () => {
              setIsConnected(false);
              console.error('WebSocket error');
            };

            wsRef.current.onclose = () => {
              setIsConnected(false);
              setTimeout(connectWebSocket, 3000);
            };
          } catch (err) {
            console.error('WebSocket connection failed:', err);
            setTimeout(connectWebSocket, 3000);
          }
        };

        connectWebSocket();
        return () => wsRef.current?.close();
      }, [wsUrl]);

      const threatColor = metrics.anomaly_score > 0.5 ? 'from-red-900 to-red-700' : 'from-green-900 to-green-700';
      const threatTextColor = metrics.anomaly_score > 0.5 ? 'text-red-400' : 'text-green-400';
      const borderColor = metrics.anomaly_score > 0.5 ? 'border-red-600' : 'border-green-600';
      const statusIcon = metrics.anomaly_score > 0.5 ? '🔴' : '🟢';

      return (
        <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 p-6 font-mono">
          <div className="mb-8">
            <div className="flex items-center justify-between mb-4">
              <h1 className="text-2xl font-bold text-cyan-400">🤖 UNIFIED MAMBA INFERENCE LOOP</h1>
              <div className="flex items-center gap-3">
                <div className={\`w-3 h-3 rounded-full \${isConnected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}\`} />
                <span className={\`text-sm \${isConnected ? 'text-green-400' : 'text-red-400'}\`}>
                  {isConnected ? 'LIVE' : 'CONNECTING'}
                </span>
              </div>
            </div>
            <p className="text-slate-500 text-xs">Real-time neural waveshaper metrics</p>
          </div>

          <div className="grid grid-cols-12 gap-4">
            <div className={\`col-span-3 bg-gradient-to-br \${threatColor} rounded-lg border \${borderColor} p-6\`}>
              <div className="text-slate-400 text-xs uppercase mb-2">Threat Level</div>
              <div className={\`text-5xl font-black \${threatTextColor} font-mono\`}>
                {(metrics.anomaly_score * 100).toFixed(1)}%
              </div>
              <div className="w-full bg-slate-800 rounded-full h-2 mt-4 overflow-hidden">
                <div
                  className={\`h-full transition-all duration-300 \${
                    metrics.anomaly_score > 0.5 ? 'bg-red-500' : 'bg-green-500'
                  }\`}
                  style={{ width: \`\${metrics.anomaly_score * 100}%\` }}
                />
              </div>
            </div>

            <div className="col-span-9 bg-slate-800 rounded-lg border border-slate-700 p-6">
              <div className={\`text-3xl font-bold text-white mb-2\`}>
                {statusIcon} {metrics.isUnderAttack ? 'DEFENSE ACTIVE' : 'MONITORING'}
              </div>
              <div className="text-sm text-slate-300 mb-6">
                Frame: {metrics.frame_index} | Samples: {metrics.total_samples}
              </div>

              <div className="grid grid-cols-3 gap-4">
                {[
                  { name: 'Drive', value: metrics.drive },
                  { name: 'Foldback', value: metrics.foldback },
                  { name: 'Asymmetry', value: metrics.asymmetry },
                ].map((param) => (
                  <div key={param.name} className="bg-slate-900 rounded p-3">
                    <div className="text-xs text-slate-400 mb-2">{param.name}</div>
                    <div className="text-2xl font-bold text-green-400">
                      {(param.value * 100).toFixed(0)}%
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <div className="mt-6 bg-slate-800 rounded-lg border border-slate-700 p-4 text-xs text-slate-400">
            <p>Multi-rate signal fusion • C925e @ 32kHz + SDR @ 6.144MHz • Native Δt preservation</p>
          </div>
        </div>
      );
    };

    ReactDOM.createRoot(document.getElementById('root')).render(
      React.createElement(WaveshaperMetrics)
    );
  </script>
</body>
</html>`;
}
