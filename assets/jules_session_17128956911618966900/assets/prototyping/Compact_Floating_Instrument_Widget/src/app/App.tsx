import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { Settings, Play, Square, Download, Activity, Radar, Zap } from 'lucide-react';
import '../styles/fonts.css';

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

// ── Typography ──────────────────────────────────────────────────
const FONT_PRIMARY = "'Inter', system-ui, sans-serif";
const FONT_MONO = "'Space Mono', 'Courier New', monospace";

// ── Frequency to Color Mapping (Harmonic Palette) ──────────────────
function freqToColor(freqHz: number): string {
  if (freqHz < 1_000) {
    return C.red;      // Below 1 kHz → Red
  } else if (freqHz < 1_000_000) {
    return C.teal;     // Below 1 MHz → Teal
  } else {
    return C.violet;   // 1MHz+ → Violet
  }
}

export default function App() {
  const [dominantFreqHz, setDominantFreqHz] = useState(2_400_000_000);
  const [isExpanded, setIsExpanded] = useState(false);

  // Cycle through frequency colors every 2 seconds (demo mode)
  useEffect(() => {
    const interval = setInterval(() => {
      setDominantFreqHz((prev) => {
        if (prev === 2_400_000_000) return 100_000;
        if (prev === 100_000) return 60;
        return 2_400_000_000;
      });
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-[#07080F] flex items-center justify-center p-4 sm:p-8 font-sans text-sm selection:bg-teal-500/30">
      
      {/* 
        SPATIAL ROOT ELEMENT
        We use w-full max-w-sm to give it a base size (e.g. 24rem/384px wide).
        aspect-[1.6] enforces the credit card shape in collapsed state.
        If expanded, we let it grow in height dynamically.
      */}
      <motion.div
        animate={{ 
          aspectRatio: isExpanded ? 'auto' : '1.6'
        }}
        transition={{ duration: 0.4, ease: [0.25, 1, 0.5, 1] }}
        className="w-full max-w-[24rem] sm:max-w-[26rem]"
        style={{
          background: C.bg,
          borderRadius: '0.75rem',
          border: `1px solid ${C.border}`,
          boxShadow: '0 10px 30px -10px rgba(0,0,0,0.8), inset 0 1px 0 rgba(255,255,255,0.05)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
          position: 'relative',
        }}
      >
        <div style={{ padding: '0.5rem', display: 'flex', flexDirection: 'column', gap: '0.375rem', flex: 1 }}>
          
          {/* ════════════════════════════════════════════════════════
              HEADER PANEL
          ════════════════════════════════════════════════════════ */}
          <div
            style={{
              height: '2.5rem',
              background: C.panelBg,
              border: `1px solid ${C.border}`,
              borderRadius: '0.375rem',
              padding: '0 0.75rem',
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              flexShrink: 0,
            }}
          >
            {/* Left side */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
              <div style={{ color: C.teal, fontFamily: FONT_PRIMARY, fontWeight: 700, fontSize: '0.85em', letterSpacing: '0.05em' }}>
                TOTO CORE
              </div>
              <div style={{ width: 1, height: '1rem', background: C.border }} />
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
                <span style={{ color: C.grey, fontFamily: FONT_PRIMARY, fontWeight: 500, fontSize: '0.75em' }}>Anomaly Score</span>
                <span style={{ color: freqToColor(dominantFreqHz), fontFamily: FONT_MONO, fontSize: '0.8em', fontWeight: 'bold', transition: 'color 0.4s' }}>0.150</span>
              </div>
            </div>

            {/* Right side */}
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <div className="hidden sm:block" style={{ color: C.teal, fontFamily: FONT_PRIMARY, fontWeight: 500, fontSize: '0.75em', opacity: 0.8 }}>
                Neural Auto-Steer
              </div>
              {/* Custom Toggle Switch */}
              <div
                style={{
                  width: '1.75rem',
                  height: '0.875rem',
                  borderRadius: '1rem',
                  background: `${C.teal}40`,
                  border: `1px solid ${C.teal}`,
                  position: 'relative',
                  cursor: 'pointer',
                  boxShadow: `0 0 5px ${C.teal}40`,
                }}
              >
                <div
                  style={{
                    width: '0.625rem',
                    height: '0.625rem',
                    borderRadius: '50%',
                    background: C.teal,
                    position: 'absolute',
                    top: '0.0625rem',
                    right: '0.125rem',
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
              flex: 1,
              minHeight: '7rem', // Ensure it doesn't collapse too far
              background: C.panelBg,
              border: `1px solid ${C.border}`,
              borderRadius: '0.375rem',
              position: 'relative',
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden',
            }}
          >
            {/* Top Labels */}
            <div style={{ display: 'flex', justifyContent: 'space-between', padding: '0.5rem 0.625rem', zIndex: 10 }}>
              <div style={{ color: C.teal, fontFamily: FONT_PRIMARY, fontWeight: 500, fontSize: '0.75em' }}>
                WgpuShaderZone (BSS)
              </div>
              <div style={{ color: freqToColor(dominantFreqHz), fontFamily: FONT_PRIMARY, fontWeight: 500, fontSize: '0.75em', transition: 'color 0.4s' }}>
                {dominantFreqHz < 1000 ? "60Hz Cluster" : dominantFreqHz < 1_000_000 ? "85kHz Cluster" : "2.4GHz Cluster"}
              </div>
            </div>

            {/* Graph Area */}
            <div style={{ flex: 1, position: 'relative' }}>
              {/* Grid Lines */}
              <svg width="100%" height="100%" style={{ position: 'absolute', top: 0, left: 0 }}>
                <pattern id="grid" width="2rem" height="2rem" patternUnits="userSpaceOnUse">
                  <path d="M 32 0 L 0 0 0 32" fill="none" stroke={C.grid} strokeWidth="1" />
                </pattern>
                <rect width="100%" height="100%" fill="url(#grid)" />
                {/* Center horizontal line */}
                <line x1="0" y1="50%" x2="100%" y2="50%" stroke={C.border} strokeWidth="1" />
              </svg>

              {/* Glowing Path (Rainbow Smear placeholder) */}
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
                <motion.path
                  initial={false}
                  animate={{ stroke: freqToColor(dominantFreqHz) }}
                  transition={{ duration: 0.4 }}
                  d="M 0 50 C 20 20, 40 80, 60 50 C 80 20, 90 90, 110 50 C 130 10, 150 10, 170 50 C 190 90, 210 90, 230 50 C 250 10, 270 10, 290 50 C 310 90, 320 50, 320 50"
                  fill="none"
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
              paddingBottom: '0.375rem',
              zIndex: 10
            }}>
              <div style={{ color: C.teal, fontFamily: FONT_PRIMARY, fontWeight: 500, fontSize: '0.7em', opacity: 0.7 }}>
                Time (10ms)
              </div>
            </div>
          </div>

          {/* ════════════════════════════════════════════════════════
              TELEMETRY STRIP (LEARNING LOSS + DVR)
          ════════════════════════════════════════════════════════ */}
          <div
            style={{
              height: '3rem',
              display: 'flex',
              gap: '0.375rem',
              flexShrink: 0,
            }}
          >
            {/* DVR Status */}
            <div
              style={{
                width: '7.5rem',
                background: C.panelBg,
                border: `1px solid ${C.border}`,
                borderRadius: '0.375rem',
                padding: '0.375rem 0.625rem',
                display: 'flex',
                flexDirection: 'column',
                justifyContent: 'center',
                gap: '0.25rem'
              }}
            >
               <div style={{ display: 'flex', alignItems: 'center', gap: '0.375rem' }}>
                  <div style={{ position: 'relative', width: '0.5rem', height: '0.5rem' }}>
                    <div style={{ position: 'absolute', width: '100%', height: '100%', borderRadius: '50%', backgroundColor: C.red, opacity: 0.8 }} />
                    <motion.div
                      animate={{ scale: [1, 1.5, 1], opacity: [0.8, 0, 0.8] }}
                      transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
                      style={{ position: 'absolute', width: '100%', height: '100%', borderRadius: '50%', backgroundColor: C.red }}
                    />
                  </div>
                  <span style={{ color: C.white, fontFamily: FONT_PRIMARY, fontWeight: 700, fontSize: '0.75em', letterSpacing: '0.05em' }}>DVR: REC</span>
               </div>
               <div style={{ color: C.grey, fontFamily: FONT_MONO, fontSize: '0.65em' }}>
                 Buffer: 97 Days
               </div>
            </div>

            {/* Miniaturized Learning Loss Chart */}
            <div
              style={{
                flex: 1,
                background: C.panelBg,
                border: `1px solid ${C.border}`,
                borderRadius: '0.375rem',
                padding: '0.25rem 0.5rem',
                display: 'flex',
                flexDirection: 'column',
                position: 'relative',
                overflow: 'hidden'
              }}
            >
               <div style={{ display: 'flex', justifyContent: 'space-between', zIndex: 10 }}>
                  <span style={{ color: C.teal, fontFamily: FONT_PRIMARY, fontWeight: 500, fontSize: '0.7em' }}>Learning Loss</span>
                  <span style={{ color: C.white, fontFamily: FONT_MONO, fontSize: '0.7em' }}>0.042</span>
               </div>
               
               <div style={{ flex: 1, position: 'relative', marginTop: '0.125rem' }}>
                  <svg width="100%" height="100%" viewBox="0 0 100 20" preserveAspectRatio="none" style={{ position: 'absolute', top: 0, left: 0 }}>
                    <motion.path
                      d="M 0 18 L 10 16 L 20 17 L 30 12 L 40 14 L 50 8 L 60 9 L 70 5 L 80 6 L 90 2 L 100 3"
                      fill="none"
                      stroke={C.teal}
                      strokeWidth="1.5"
                      vectorEffect="non-scaling-stroke"
                    />
                    <path
                      d="M 0 18 L 10 16 L 20 17 L 30 12 L 40 14 L 50 8 L 60 9 L 70 5 L 80 6 L 90 2 L 100 3 L 100 20 L 0 20 Z"
                      fill={`${C.teal}20`}
                      stroke="none"
                    />
                  </svg>
               </div>
            </div>

            {/* Gear Icon / Expander */}
            <button
              onClick={() => setIsExpanded(!isExpanded)}
              style={{
                width: '3rem',
                background: isExpanded ? C.border : C.panelBg,
                border: `1px solid ${C.border}`,
                borderRadius: '0.375rem',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                cursor: 'pointer',
                transition: 'background 0.2s',
                color: isExpanded ? C.white : C.grey
              }}
            >
              <Settings size={'1.125rem'} />
            </button>
          </div>

          {/* ════════════════════════════════════════════════════════
              SETTINGS PANE (EXPANDABLE)
          ════════════════════════════════════════════════════════ */}
          <AnimatePresence>
            {isExpanded && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: 'auto' }}
                exit={{ opacity: 0, height: 0 }}
                transition={{ duration: 0.3 }}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '0.375rem',
                  overflow: 'hidden'
                }}
              >
                <div style={{ height: '0.125rem' }} /> {/* Slight margin */}
                
                {/* Active Denial Section */}
                <div style={{ background: C.panelBg, border: `1px solid ${C.border}`, borderRadius: '0.375rem', padding: '0.75rem', flex: 1, display: 'flex', flexDirection: 'column' }}>
                   <div style={{ display: 'flex', alignItems: 'center', gap: '0.375rem', marginBottom: '0.625rem' }}>
                     <Zap size={'0.875rem'} color={C.violet} />
                     <span style={{ color: C.white, fontFamily: FONT_PRIMARY, fontWeight: 600, fontSize: '0.8em' }}>Active Denial</span>
                   </div>

                   <div style={{ display: 'flex', flexDirection: 'column', gap: '0.375rem' }}>
                     <DenialItem label="2.4GHz WiFi Jam" freq="2450 MHz" active />
                     <DenialItem label="Bluetooth BLE Disruption" freq="2402 MHz" active={false} />
                     <DenialItem label="UHF Drone Link" freq="433 MHz" active={false} />
                   </div>
                </div>

                {/* Probe Buttons */}
                <div style={{ display: 'flex', gap: '0.375rem' }}>
                  <ProbeBtn icon={<Activity size={'1rem'} />} label="Analyze" />
                  <ProbeBtn icon={<Radar size={'1rem'} />} label="Sweep" />
                  <ProbeBtn icon={<Download size={'1rem'} />} label="Export" />
                </div>
              </motion.div>
            )}
          </AnimatePresence>

        </div>
      </motion.div>
    </div>
  );
}

// ── Subcomponents ──────────────────────────────────────────────────

function DenialItem({ label, freq, active }: { label: string, freq: string, active: boolean }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: C.bg, padding: '0.375rem 0.625rem', borderRadius: '0.25rem', border: `1px solid ${active ? C.violet : C.border}` }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '0.125rem' }}>
        <span style={{ color: active ? C.white : C.grey, fontFamily: FONT_PRIMARY, fontSize: '0.75em', fontWeight: 500 }}>{label}</span>
        <span style={{ color: C.greyDim, fontFamily: FONT_MONO, fontSize: '0.65em' }}>{freq}</span>
      </div>
      <div style={{ width: '2rem', height: '1rem', borderRadius: '0.5rem', background: active ? `${C.violet}40` : C.panelBg, border: `1px solid ${active ? C.violet : C.greyDim}`, position: 'relative', cursor: 'pointer' }}>
        <div style={{ width: '0.75rem', height: '0.75rem', borderRadius: '50%', background: active ? C.violet : C.greyDim, position: 'absolute', top: '0.0625rem', right: active ? '0.125rem' : 'auto', left: active ? 'auto' : '0.125rem' }} />
      </div>
    </div>
  )
}

function ProbeBtn({ icon, label }: { icon: React.ReactNode, label: string }) {
  return (
    <button style={{ flex: 1, height: '2.25rem', background: C.panelBg, border: `1px solid ${C.border}`, borderRadius: '0.375rem', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.375rem', cursor: 'pointer', color: C.grey, transition: 'all 0.2s' }}>
      {icon}
      <span style={{ fontFamily: FONT_PRIMARY, fontSize: '0.75em', fontWeight: 500 }}>{label}</span>
    </button>
  )
}
