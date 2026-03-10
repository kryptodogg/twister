# 🤖 Waveshaper Metrics - Live Neural Inference Applet

A beautiful, real-time React visualization of the Mamba neural network's harassment defense inference loop.

## 📊 Features

- **Live Threat Metrics**: Real-time anomaly score display with color-coded threat levels
- **Waveform Oscilloscope**: Super-Nyquist harmonic distortion visualization
- **Latent Activity Monitor**: 128-dimensional embedding activity visualization (Drive, Foldback, Asymmetry)
- **Neural Auto-Steer Toggle**: Switch between AI-controlled and manual parameter modes
- **Parameter Sliders**: Real-time adjustment of Drive, Foldback, and Asymmetry parameters
- **Anomaly Timeline**: Historical chart showing threat trends and parameter evolution
- **Multi-Rate Status**: Display native sample rate information (C925e @ 32kHz, SDR @ 6.144MHz)
- **WebSocket Streaming**: Lock-free 60FPS metrics streaming from Rust backend

## 🚀 Quick Start

### Option 1: Development Mode (Recommended)

```bash
# Install dependencies
cd ui/applets
npm install

# Terminal 1: Start the metrics server (bridges Rust backend to React UI)
npm run server

# Terminal 2: Start Vite dev server
npm run dev

# Open http://localhost:5173 in your browser
```

### Option 2: Production Build

```bash
# Build optimized bundle
npm run build

# Output is in dist/ - serve with any HTTP server
npx serve dist

# Open http://localhost:3000 in your browser
```

### Option 3: Standalone Server

The metrics server includes a fallback HTML page that doesn't require npm:

```bash
# Just run the server
node ui/applets/metrics-server.mjs

# Open http://localhost:8080 in your browser
```

## 🔌 Architecture

```
Rust Backend (Twister)
      ↓
  (HTTP/IPC)
      ↓
Metrics Server (Node.js)
      ├─ Polls Rust backend every 16ms (~60 FPS)
      ├─ Generates simulated metrics if backend unavailable
      └─ Broadcasts via WebSocket
      ↓
React Frontend (Vite + React 18)
      ├─ WebSocket listener (ws://localhost:8080/metrics)
      ├─ State management (React hooks)
      └─ Real-time UI updates (Recharts, Tailwind)
```

## 📋 Metrics Streamed

```json
{
  "anomaly_score": 0.42,           // 0.0-1.0 threat level
  "drive": 0.0,                    // 0.0-1.0 waveshaping drive
  "foldback": 0.0,                 // 0.0-1.0 harmonic foldback
  "asymmetry": 0.5,                // -1.0-1.0 asymmetric distortion
  "latent_activity": [0.42, 0.0, 0.5],  // 128D embedding (pooled to 3 chunks)
  "waveform": [0.0, 0.1, ...],    // 100-sample oscilloscope data
  "frame_index": 12345,            // Frame counter
  "total_samples": 27520          // Total samples processed
}
```

## 🎨 Design Highlights

- **Dark Mode**: Slate-900/950 background with neon accent colors (cyan, green, red)
- **Real-Time Animations**: Smooth progress bars, color transitions, and metrics updates
- **Responsive Layout**: Adapts to desktop, tablet, and mobile viewports
- **Accessibility**: Clear typography, high contrast, keyboard-navigable controls
- **Performance**: 60 FPS WebSocket streaming, efficient React rendering with hooks

## 🔧 Configuration

### Environment Variables

```bash
# Metrics server configuration
RUST_BACKEND_URL=http://localhost:3000  # Backend API endpoint
PORT=8080                               # WebSocket server port

# Development
npm run dev              # Vite dev server (port 5173)
npm run build            # Production build
npm run preview          # Preview production build locally
```

### WebSocket Connection

The React component connects to `ws://localhost:8080/metrics` by default.
Customize by passing the `wsUrl` prop:

```jsx
<WaveshaperMetrics wsUrl="ws://your-server:8080/metrics" />
```

## 🧩 Component Structure

```
WaveshaperMetrics (Main Component)
├── Connection Status Indicator
├── Threat Metric Display
│   ├── Anomaly Score (large percentage)
│   └── Progress bar
├── Waveform Oscilloscope
│   └── SVG-rendered waveform with gridlines
├── Status Indicator
│   └── Attack/Monitoring state
├── Neural Control Panel
│   ├── Auto-Steer toggle
│   ├── Latent activity bars
│   └── Latent dimensions visualization
├── Parameter Sliders (Drive/Foldback/Asymmetry)
└── Anomaly Timeline Chart
    └─ Recharts line chart
```

## 📦 Dependencies

### Core
- **React 18**: UI framework
- **React DOM 18**: DOM rendering
- **Recharts**: Data visualization (line charts)

### Development
- **Vite**: Build tool & dev server
- **Tailwind CSS**: Utility-first styling
- **@vitejs/plugin-react**: Fast refresh

### Backend
- **ws**: WebSocket server (Node.js)
- **http**: HTTP server (Node.js stdlib)

## 🐛 Troubleshooting

### WebSocket Connection Failed
1. Ensure metrics server is running: `node metrics-server.mjs`
2. Check port 8080 is available: `lsof -i :8080` (macOS/Linux) or `netstat -ano | findstr :8080` (Windows)
3. Verify firewall allows localhost connections

### Metrics Not Updating
1. Check browser console for WebSocket errors
2. Verify Rust backend is running on localhost:3000
3. If backend unavailable, server will use simulated metrics (confirm in console logs)

### Build Errors
```bash
# Clear cache and reinstall
rm -rf node_modules package-lock.json
npm install
npm run build
```

## 📈 Performance

- **60 FPS WebSocket streaming**: 16ms update interval
- **React re-renders**: Only when metrics change (efficient subscriptions)
- **Chart performance**: Recharts optimized for real-time data
- **Bundle size**: ~180KB gzipped (React + Recharts + dependencies)

## 🔐 Security Notes

- WebSocket connection is unencrypted (localhost only)
- For production, use WSS (WebSocket Secure)
- Backend authentication should be added before exposing to network

## 📄 License

Part of the Twister harassment detection system. See parent repo for license.

## 🙋 Support

For issues or feature requests, see the main Twister repository.
