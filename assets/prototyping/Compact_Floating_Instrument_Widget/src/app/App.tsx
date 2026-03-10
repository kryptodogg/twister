import React, { useState, useEffect } from 'react';
import { motion } from 'motion/react';
import '../styles/fonts.css';

// ── Widget dimensions ──────────────────────────────────────────────
const W = 336;
const H = 212;
const H_HEADER = Math.round(H * 0.19);    // 40px
const H_CANVAS = Math.round(H * 0.52);    // 110px
const H_TELEMETRY = Math.round(H * 0.29); // 62px
const PAD = 6;
const GAP = 4;

// ── Colour palette ────────────────────────────────────────────────
const C = {
  bg: '#0F111A',
  panelBg: '#0A0C14',
  border: '#2A2E3D',
  grid: '#1F2233',
  teal: '#00E5C8',
  violet: '#A855F7',
  green: '#22C55E',
  grey: '#9CA3AF',
  greyDim: '#4B5563',
  white: '#FFFFFF',
  red: '#FF1A1A',
};

const MONO = "'Space Mono', 'Courier New', monospace";
const SANS = "system-ui, -apple-system, sans-serif";

// ── Frequency to Color Mapping (Harmonic Palette) ──────────────────
function freqToColor(freqHz: number): string {
  if (freqHz < 1_000) {
    return C.red;      // Below 1 kHz → Red (60Hz, ELF, low audio)
  } else if (freqHz < 1_000_000) {
    return C.teal;     // Below 1 MHz → Teal (ultrasonic, folded audio)
  } else {
    return C.violet;   // 1MHz+ → Violet (RF, 2.4GHz, microwave)
  }
}

export default function App() {
  const [dominantFreqHz, setDominantFreqHz] = useState(2_400_000_000);

  // Cycle through frequency colors every 2 seconds (demo mode)
  useEffect(() => {
    const interval = setInterval(() => {
      setDominantFreqHz((prev) => {
        if (prev === 2_400_000_000) return 100_000;      // 2.4GHz → 100kHz
        if (prev === 100_000) return 60;                 // 100kHz → 60Hz
        return 2_400_000_000;                             // 60Hz → 2.4GHz
      });
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-[#07080F] flex items-center justify-center p-4 selection:bg-teal-500/30">
      <div
        style={{
          width: W,
          height: H,
          background: C.bg,
          borderRadius: 8,
          border: `1px solid ${C.border}`,
          boxShadow: '0 10px 30px -10px rgba(0,0,0,0.8), inset 0 1px 0 rgba(255,255,255,0.05)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
          position: 'relative',
        }}
      >
        <div style={{ padding: PAD, display: 'flex', flexDirection: 'column', gap: GAP, flex: 1 }}>
          
          {/* ════════════════════════════════════════════════════════
              HEADER PANEL
          ════════════════════════════════════════════════════════ */}
          <div
            style={{
              height: H_HEADER,
              background: C.panelBg,
              border: `1px solid ${C.border}`,
              borderRadius: 5,
              padding: '6px 8px',
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              flexShrink: 0,
            }}
          >
            {/* Left side */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <div style={{ color: C.teal, fontFamily: MONO, fontSize: 8.5, letterSpacing: '0.05em' }}>
                Toto
              </div>
                <div style={{ color: C.grey, fontFamily: MONO, fontSize: 8 }}>
                  Anomaly Score: <span style={{ color: C.violet }}>0.150</span>
                </div>
            </div>

            {/* Right side */}
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div style={{ color: C.teal, fontFamily: MONO, fontSize: 7.5, opacity: 0.8 }}>
                Neural Auto-Steer: Active
              </div>
              {/* Custom Toggle Switch */}
              <div
                style={{
                  width: 24,
                  height: 12,
                  borderRadius: 12,
                  background: `${C.teal}40`,
                  border: `1px solid ${C.teal}`,
                  position: 'relative',
                  cursor: 'pointer',
                  boxShadow: `0 0 5px ${C.teal}40`,
                }}
              >
                <div
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: '50%',
                    background: C.teal,
                    position: 'absolute',
                    top: 1,
                    right: 2,
                    boxShadow: `0 0 4px ${C.teal}`,
                  }}
                />
              </div>
            </div>
          </div>

          {/* ════════════════════════════════════════════════════════
              MAIN CANVAS (OSCILLOSCOPE)
          ════════════════════════════════════════════════════════ */}
          <div
            style={{
              height: H_CANVAS,
              background: C.panelBg,
              border: `1px solid ${C.border}`,
              borderRadius: 5,
              position: 'relative',
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden',
              flexShrink: 0,
            }}
          >
            {/* Top Labels */}
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 8px', zIndex: 10 }}>
              <div style={{ color: C.teal, fontFamily: MONO, fontSize: 8 }}>
                Oscilloscope
              </div>
              <div style={{ color: C.violet, fontFamily: MONO, fontSize: 8 }}>
                Violet Cloak Harmonic Smear
              </div>
            </div>

            {/* Graph Area */}
            <div style={{ flex: 1, position: 'relative' }}>
              {/* Grid Lines */}
              <svg width="100%" height="100%" style={{ position: 'absolute', top: 0, left: 0 }}>
                <pattern id="grid" width="32" height="32" patternUnits="userSpaceOnUse">
                  <path d="M 32 0 L 0 0 0 32" fill="none" stroke={C.grid} strokeWidth="1" />
                </pattern>
                <rect width="100%" height="100%" fill="url(#grid)" />
                {/* Center horizontal line */}
                <line x1="0" y1="50%" x2="100%" y2="50%" stroke={C.border} strokeWidth="1" />
              </svg>

              {/* Glowing Path */}
              <svg width="100%" height="100%" viewBox="0 0 320 100" preserveAspectRatio="none" style={{ position: 'absolute', top: 0, left: 0 }}>
                <defs>
                  <filter id="glow">
                    <feGaussianBlur stdDeviation="2" result="coloredBlur"/>
                    <feMerge>
                      <feMergeNode in="coloredBlur"/>
                      <feMergeNode in="SourceGraphic"/>
                    </feMerge>
                  </filter>
                </defs>
                <path
                  d="M 0 50 C 20 20, 40 80, 60 50 C 80 20, 90 90, 110 50 C 130 10, 150 10, 170 50 C 190 90, 210 90, 230 50 C 250 10, 270 10, 290 50 C 310 90, 320 50, 320 50"
                  fill="none"
                  stroke={freqToColor(dominantFreqHz)}
                  strokeWidth="2.5"
                  filter="url(#glow)"
                  vectorEffect="non-scaling-stroke"
                />
              </svg>
            </div>

            {/* Bottom Label */}
            <div style={{
              display: 'flex',
              justifyContent: 'center',
              alignItems: 'center',
              paddingBottom: 4,
              zIndex: 10
            }}>
              <div style={{ color: C.teal, fontFamily: MONO, fontSize: 7, opacity: 0.7 }}>
                Time (10ms)
              </div>
            </div>
          </div>

          {/* ════════════════════════════════════════════════════════
              TELEMETRY STRIP (MAMBA PROJECTIONS - PERMANENT)
          ════════════════════════════════════════════════════════ */}
          <div
            style={{
              height: H_TELEMETRY,
              display: 'flex',
              gap: GAP,
              flexShrink: 0,
            }}
          >
            <TrainingBox
              label="Drive"
              val="0.250"
              color={C.teal}
              progress={0.30}
            />
            <TrainingBox
              label="Fold"
              val="0.700"
              color={C.green}
              progress={0.70}
            />
            <TrainingBox
              label="Asym"
              val="0.150"
              color={C.violet}
              progress={0.15}
            />
          </div>

        </div>
      </div>
    </div>
  );
}

// ── Subcomponents ──────────────────────────────────────────────────

function TrainingBox({ label, val, color, progress }: any) {
  return (
    <div
      style={{
        flex: 1,
        background: C.panelBg,
        border: `1px solid ${C.border}`,
        borderRadius: 5,
        padding: '6px',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
      }}
    >
      {/* Label and Value */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline' }}>
        <div style={{ color: C.white, fontFamily: MONO, fontSize: 8 }}>
          {label}
        </div>
        <div style={{ color: color, fontFamily: MONO, fontSize: 8, fontWeight: 'bold' }}>
          {val}
        </div>
      </div>

      {/* Progress Bar (3px) */}
      <div style={{
        width: '100%',
        height: 3,
        background: C.border,
        borderRadius: 1.5,
        overflow: 'hidden',
        marginTop: 4,
      }}>
        <div
          style={{
            width: `${progress * 100}%`,
            height: '100%',
            background: color,
            borderRadius: 1.5,
            transition: 'width 0.3s ease',
          }}
        />
      </div>
    </div>
  );
}
