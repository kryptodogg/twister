/**
 * WaveshaperMetrics.jsx
 *
 * Live neural waveshaper visualization applet
 * Real-time display of Mamba anomaly detection, latent embeddings, and waveform analysis
 *
 * Connects to backend via WebSocket for live metrics streaming
 * Updates at 60 FPS with smooth animations
 */

import React, { useState, useEffect, useRef } from 'react';
import { LineChart, Line, ResponsiveContainer, XAxis, YAxis, CartesianGrid, Tooltip } from 'recharts';

const WaveshaperMetrics = ({ wsUrl = 'ws://localhost:8080/metrics' }) => {
  // ─────────────────────────────────────────────────────────────────────────────
  // State Management
  // ─────────────────────────────────────────────────────────────────────────────

  const [metrics, setMetrics] = useState({
    anomalyScore: 0.0,
    drive: 0.0,
    foldback: 0.0,
    asymmetry: 0.5,
    latentActivity: [0.0, 0.0, 0.0],
    waveform: Array(100).fill(0),
    frameIndex: 0,
    totalSamples: 0,
    status: '🟢 MONITORING',
    isUnderAttack: false,
  });

  const [isConnected, setIsConnected] = useState(false);
  const [autoSteer, setAutoSteer] = useState(true);
  const [historyData, setHistoryData] = useState([]);
  const wsRef = useRef(null);
  const frameCountRef = useRef(0);

  // ─────────────────────────────────────────────────────────────────────────────
  // WebSocket Connection & Data Streaming
  // ─────────────────────────────────────────────────────────────────────────────

  useEffect(() => {
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

            // Update metrics state
            setMetrics((prev) => ({
              ...prev,
              anomalyScore: data.anomaly_score ?? prev.anomalyScore,
              drive: data.drive ?? prev.drive,
              foldback: data.foldback ?? prev.foldback,
              asymmetry: data.asymmetry ?? prev.asymmetry,
              latentActivity: data.latent_activity ?? prev.latentActivity,
              waveform: data.waveform ?? prev.waveform,
              frameIndex: data.frame_index ?? prev.frameIndex,
              totalSamples: data.total_samples ?? prev.totalSamples,
              isUnderAttack: (data.anomaly_score ?? 0) > 0.5,
            }));

            // Update history for chart
            frameCountRef.current++;
            if (frameCountRef.current % 5 === 0) {
              setHistoryData((prev) => [
                ...prev.slice(-59),
                {
                  frame: data.frame_index ?? 0,
                  anomaly: Math.round((data.anomaly_score ?? 0) * 100),
                  drive: Math.round((data.drive ?? 0) * 100),
                },
              ]);
            }
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
          console.log('Disconnected from metrics stream');
          // Attempt reconnect after 3 seconds
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

  // ─────────────────────────────────────────────────────────────────────────────
  // Render Oscilloscope Waveform
  // ─────────────────────────────────────────────────────────────────────────────

  const renderWaveform = () => {
    const width = 400;
    const height = 120;
    const padding = 10;
    const plotWidth = width - 2 * padding;
    const plotHeight = height - 2 * padding;
    const centerY = padding + plotHeight / 2;

    const points = metrics.waveform
      .map((value, i) => ({
        x: padding + (i / metrics.waveform.length) * plotWidth,
        y: centerY - (value * plotHeight) / 2.4,
      }))
      .filter((_, i) => i < metrics.waveform.length);

    const pathData = points
      .map((p, i) => `${i === 0 ? 'M' : 'L'} ${p.x} ${p.y}`)
      .join(' ');

    return (
      <svg width={width} height={height} className="bg-slate-900 rounded border border-slate-700">
        {/* Gridlines */}
        <line x1={padding} y1={centerY} x2={width - padding} y2={centerY} stroke="#333" strokeWidth="1" />
        {/* Waveform */}
        <path d={pathData} fill="none" stroke="#bb00ff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
        {/* Grid */}
        <g stroke="#333" strokeWidth="0.5" strokeDasharray="3,3">
          {[0.25, 0.75].map((frac) => (
            <line key={frac} x1={padding} y1={centerY - (plotHeight / 2) * frac} x2={width - padding} y2={centerY - (plotHeight / 2) * frac} />
          ))}
        </g>
      </svg>
    );
  };

  // ─────────────────────────────────────────────────────────────────────────────
  // Color & Status Indicators
  // ─────────────────────────────────────────────────────────────────────────────

  const threatColor = metrics.anomalyScore > 0.5 ? 'from-red-900 to-red-700' : 'from-green-900 to-green-700';
  const threatTextColor = metrics.anomalyScore > 0.5 ? 'text-red-400' : 'text-green-400';
  const borderColor = metrics.anomalyScore > 0.5 ? 'border-red-600' : 'border-green-600';
  const statusIcon = metrics.anomalyScore > 0.5 ? '🔴' : '🟢';

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 p-6 font-mono">
      {/* Header */}
      <div className="mb-8">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-2xl font-bold text-cyan-400">🤖 UNIFIED MAMBA INFERENCE LOOP</h1>
          <div className="flex items-center gap-3">
            <div className={`w-3 h-3 rounded-full ${isConnected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
            <span className={`text-sm ${isConnected ? 'text-green-400' : 'text-red-400'}`}>
              {isConnected ? 'LIVE' : 'DISCONNECTED'}
            </span>
          </div>
        </div>
        <p className="text-slate-500 text-xs">Multi-rate signal fusion • Real-time anomaly detection • Neural auto-steer</p>
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-12 gap-4 max-w-7xl">
        {/* Threat Metric - Left Column */}
        <div className={`col-span-3 bg-gradient-to-br ${threatColor} rounded-lg border ${borderColor} p-6 shadow-xl`}>
          <div className="text-slate-400 text-xs uppercase tracking-wider mb-2">Threat Level</div>
          <div className={`text-5xl font-black ${threatTextColor} mb-4 font-mono`}>
            {(metrics.anomalyScore * 100).toFixed(1)}%
          </div>
          <div className="w-full bg-slate-800 rounded-full h-2 mb-4 overflow-hidden">
            <div
              className={`h-full transition-all duration-300 ${
                metrics.anomalyScore > 0.5 ? 'bg-red-500' : 'bg-green-500'
              }`}
              style={{ width: `${metrics.anomalyScore * 100}%` }}
            />
          </div>
          <div className="flex justify-between text-xs text-slate-400">
            <span>Frame: {metrics.frameIndex}</span>
            <span>{metrics.totalSamples} samples</span>
          </div>
        </div>

        {/* Oscilloscope & Status - Center Column */}
        <div className="col-span-5">
          <div className="mb-4">
            <div className="text-xs uppercase text-slate-500 mb-2 tracking-wider">Waveform (Super-Nyquist Smear)</div>
            <div className="flex justify-center">
              {renderWaveform()}
            </div>
          </div>

          <div className={`bg-gradient-to-br ${threatColor} rounded-lg border ${borderColor} p-4`}>
            <div className="text-2xl font-bold text-white">
              {statusIcon} {metrics.isUnderAttack ? 'DEFENSE ACTIVE' : 'MONITORING'}
            </div>
            <div className="text-xs text-slate-300 mt-1">
              {metrics.isUnderAttack ? 'RF interference detected • Harmonic distortion active' : 'System nominal • Ready to intercept'}
            </div>
          </div>
        </div>

        {/* Neural Control Panel - Right Column */}
        <div className="col-span-4 space-y-4">
          {/* Auto-Steer Toggle */}
          <div className="bg-slate-800 rounded-lg border border-slate-700 p-4">
            <div className="flex items-center justify-between mb-4">
              <label className="text-sm font-bold text-cyan-400">Neural Auto-Steer</label>
              <button
                onClick={() => setAutoSteer(!autoSteer)}
                className={`relative inline-flex h-7 w-12 items-center rounded-full transition-colors ${
                  autoSteer ? 'bg-cyan-600' : 'bg-slate-600'
                }`}
              >
                <span
                  className={`inline-block h-5 w-5 transform rounded-full bg-white transition-transform ${
                    autoSteer ? 'translate-x-6' : 'translate-x-1'
                  }`}
                />
              </button>
            </div>
            <p className="text-xs text-slate-400">
              {autoSteer ? 'AI controls parameters' : 'Manual slider control'}
            </p>
          </div>

          {/* Latent Activity Visualization */}
          <div className="bg-slate-800 rounded-lg border border-slate-700 p-4">
            <div className="text-xs uppercase text-slate-500 mb-3 tracking-wider">128D Latent Activity</div>
            {[
              { name: 'Drive [0..31]', value: metrics.latentActivity[0] ?? 0 },
              { name: 'Foldback [32..63]', value: metrics.latentActivity[1] ?? 0 },
              { name: 'Asymmetry [64..95]', value: metrics.latentActivity[2] ?? 0 },
            ].map((item) => (
              <div key={item.name} className="mb-3">
                <div className="flex justify-between items-center mb-1">
                  <span className="text-xs text-slate-400">{item.name}</span>
                  <span className="text-xs font-bold text-cyan-400">
                    {(item.value * 100).toFixed(0)}%
                  </span>
                </div>
                <div className="w-full bg-slate-900 rounded-full h-2 overflow-hidden">
                  <div
                    className="h-full bg-gradient-to-r from-cyan-500 to-green-500 transition-all duration-200"
                    style={{ width: `${item.value * 100}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Parameter Sliders - Full Width */}
      <div className="mt-6 grid grid-cols-3 gap-4 max-w-7xl">
        {[
          { name: 'Drive', value: metrics.drive, unit: '%' },
          { name: 'Foldback', value: metrics.foldback, unit: '%' },
          { name: 'Asymmetry', value: (metrics.asymmetry - 0.5) * 200, unit: '%' },
        ].map((param) => (
          <div key={param.name} className="bg-slate-800 rounded-lg border border-slate-700 p-4">
            <div className="flex justify-between items-center mb-3">
              <label className="text-sm font-bold text-slate-300">{param.name}</label>
              <span className="text-lg font-bold text-green-400">
                {(param.value * 100).toFixed(0)}{param.unit}
              </span>
            </div>
            <input
              type="range"
              min="0"
              max="100"
              value={param.value * 100}
              disabled={autoSteer}
              onChange={() => {}}
              className={`w-full h-2 rounded-lg cursor-pointer appearance-none bg-slate-900 ${
                autoSteer ? 'opacity-60 cursor-not-allowed' : ''
              }`}
            />
            <style>{`
              input[type='range']::-webkit-slider-thumb {
                appearance: none;
                width: 16px;
                height: 16px;
                border-radius: 50%;
                background: #06b6d4;
                cursor: pointer;
              }
              input[type='range']::-moz-range-thumb {
                width: 16px;
                height: 16px;
                border-radius: 50%;
                background: #06b6d4;
                cursor: pointer;
                border: none;
              }
            `}</style>
          </div>
        ))}
      </div>

      {/* History Chart */}
      <div className="mt-6 bg-slate-800 rounded-lg border border-slate-700 p-4 max-w-7xl">
        <div className="text-xs uppercase text-slate-500 mb-4 tracking-wider">Anomaly Timeline</div>
        <ResponsiveContainer width="100%" height={200}>
          <LineChart data={historyData}>
            <CartesianGrid strokeDasharray="3,3" stroke="#333" />
            <XAxis dataKey="frame" stroke="#666" />
            <YAxis stroke="#666" domain={[0, 100]} />
            <Tooltip
              contentStyle={{ backgroundColor: '#1e293b', border: '1px solid #475569' }}
              labelStyle={{ color: '#cbd5e1' }}
            />
            <Line
              type="monotone"
              dataKey="anomaly"
              stroke="#ff0055"
              dot={false}
              strokeWidth={2}
              isAnimationActive={false}
            />
            <Line
              type="monotone"
              dataKey="drive"
              stroke="#00ff88"
              dot={false}
              strokeWidth={2}
              isAnimationActive={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>

      {/* Architecture Info Footer */}
      <div className="mt-6 bg-slate-800 rounded-lg border border-slate-700 p-4 max-w-7xl">
        <div className="text-xs uppercase text-cyan-400 font-bold mb-2 tracking-wider">Multi-Rate Signal Fusion (Native Δt)</div>
        <p className="text-xs text-slate-400 leading-relaxed">
          Mamba SSM integrates disparate streams (C925e @ 32kHz, SDR @ 6.144MHz) via continuous differential equations.
          Native sample rates preserved without upsampling hallucination. Real-time phase tracking enables detection of
          harmonically-distorted RF interference patterns.
        </p>
      </div>
    </div>
  );
};

export default WaveshaperMetrics;
