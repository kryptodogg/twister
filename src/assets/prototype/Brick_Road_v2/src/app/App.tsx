import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import {
  Menu,
  GridView,
  Settings,
  Sensors,
  Camera,
  GraphicEq,
  Close,
  Minimize,
  CropSquare,
  Refresh,
  CheckCircle,
  Warning,
  PowerSettingsNew,
  Memory,
  DeviceHub
} from '@mui/icons-material';
import { BarChart, Bar, Cell, ResponsiveContainer, LineChart, Line } from 'recharts';

// --- MOCK DATA GENERATORS ---
const generateSpectrum = () => Array.from({ length: 40 }, (_, i) => {
  const center = 20;
  const distance = Math.abs(i - center);
  const base = Math.max(0, 100 - (distance * distance * 0.8));
  return {
    index: i,
    value: base + Math.random() * 25 + (i === 18 ? 40 : 0) + (i === 22 ? 30 : 0)
  };
});

const generateWaveform = () => Array.from({ length: 30 }, (_, i) => ({
  time: i,
  val: Math.sin(i * 0.4) * 40 + Math.random() * 15
}));

// Colors for the spectrum (MD3 Tonal Cyans & Purples)
const getSpectrumColor = (val: number) => {
  if (val > 110) return '#FFB4AB'; // Error/Red Tonal
  if (val > 80) return '#D0BCFF';  // Primary Purple Tonal
  if (val > 50) return '#80D8E1';  // Secondary Cyan Tonal
  return '#4A4458'; // Surface Variant
};

// --- COMPONENTS ---

const MD3Card = ({ children, className = '' }: { children: React.ReactNode, className?: string }) => (
  <div className={`bg-[#2B2930]/40 backdrop-blur-3xl border border-white/5 rounded-[28px] p-5 shadow-lg transition-all ${className}`}>
    {children}
  </div>
);

const MD3Switch = ({ checked, onChange, label, sublabel }: { checked: boolean, onChange: () => void, label: string, sublabel?: string }) => (
  <div className="flex items-center justify-between gap-4 w-full cursor-pointer" onClick={onChange}>
    <div className="flex flex-col">
       <span className="text-[16px] text-[#E6E1E5] font-medium tracking-wide">{label}</span>
       {sublabel && <span className="text-[12px] text-[#CAC4D0]">{sublabel}</span>}
    </div>
    <div className={`w-[52px] h-[32px] rounded-full p-[2px] transition-colors relative border shrink-0 ${checked ? 'bg-[#D0BCFF] border-[#D0BCFF]' : 'bg-[#36343B] border-[#938F99]'}`}>
      <motion.div 
        layout
        className={`h-[26px] w-[26px] rounded-full shadow-sm flex items-center justify-center ${checked ? 'bg-[#381E72]' : 'bg-[#938F99]'}`}
        style={{ marginLeft: checked ? '20px' : '0' }}
      >
        {checked && <div className="w-[10px] h-[10px] rounded-full bg-[#D0BCFF]" />}
      </motion.div>
    </div>
  </div>
);

// --- MAIN APP COMPONENT ---

export default function App() {
  const [bootState, setBootState] = useState<'booting' | 'ready' | 'testing'>('booting');
  const [bootLogs, setBootLogs] = useState<string[]>([]);
  const [spectrumData, setSpectrumData] = useState(generateSpectrum());
  const [waveData, setWaveData] = useState(generateWaveform());
  const [audioLevel, setAudioLevel] = useState(0);
  const [activeTab, setActiveTab] = useState('grid');
  const [mode, setMode] = useState('Bistatic');
  const [clockSync, setClockSync] = useState(true);
  
  const [testResults, setTestResults] = useState<any[]>([]);
  const [testProgress, setTestProgress] = useState(0);

  // Updated Hardware Stack
  const detectedHardware = [
    { id: 'pico2', name: 'Pico 2 (RP2350) Master Clock', addr: 'USB:PIO' },
    { id: 'coil', name: 'Tel. Coil Magnetometer', addr: 'Line-In' },
    { id: 'c925e_mic', name: 'C925e Raw Stereo', addr: 'USB:Audio' },
    { id: 'rtlsdr', name: 'RTL-SDR + Youloop', addr: 'USB:0BDA' },
    { id: 'pluto', name: 'PlutoSDR+ (2TX/2RX)', addr: 'USB:0456' },
    { id: 'ov9281', name: 'OV9281 Dual Stereo', addr: 'CSI/USB' },
    { id: 'c925e_vid', name: 'C925e Visual Mic', addr: 'USB:Video' },
    { id: 'ir_array', name: 'IR Emitter/Rx Array', addr: 'Pico:PIO' }
  ];

  // Boot Sequence Simulation
  useEffect(() => {
    if (bootState !== 'booting') return;

    const sequence = [
      "Initializing Synesthesia Hardware Console...",
      "Syncing Master Clock (Pico 2 RP2350 PPS)...",
      "Scanning sensor buses...",
      ...detectedHardware.map(hw => `[OK] Found ${hw.name} at ${hw.addr}`),
      "Calibrating IR Emitter Array...",
      "Hardware auto-population complete."
    ];

    let delay = 0;
    sequence.forEach((log, index) => {
      delay += 300 + Math.random() * 250;
      setTimeout(() => {
        setBootLogs(prev => [...prev, log]);
        if (index === sequence.length - 1) {
          setTimeout(() => setBootState('ready'), 800);
        }
      }, delay);
    });
  }, [bootState]);

  // Live Data Simulation
  useEffect(() => {
    if (bootState !== 'ready') return;

    const interval = setInterval(() => {
      setSpectrumData(generateSpectrum());
      setWaveData(generateWaveform());
      setAudioLevel(Math.random() * 100);
    }, 200);

    return () => clearInterval(interval);
  }, [bootState]);

  // Run Testing Suite
  const runDiagnostics = () => {
    setBootState('testing');
    setTestResults([]);
    setTestProgress(0);

    let progress = 0;
    const testInterval = setInterval(() => {
      progress += 4;
      setTestProgress(progress);

      const hwIndex = Math.floor((progress / 100) * detectedHardware.length);
      
      if (progress % 12 === 0 && hwIndex < detectedHardware.length) {
         setTestResults(prev => {
            const hw = detectedHardware[prev.length];
            if (!hw) return prev;
            return [...prev, { ...hw, status: Math.random() > 0.95 ? 'WARN' : 'PASS', latency: `${(Math.random() * 2 + 0.05).toFixed(2)}ms` }];
         });
      }

      if (progress >= 100) clearInterval(testInterval);
    }, 120);
  };

  if (bootState === 'booting') {
    return (
      <div className="min-h-screen bg-[#141218] flex flex-col items-center justify-center p-8 font-inter text-[#E6E1E5] relative overflow-hidden selection:bg-[#D0BCFF]/30">
        <div className="w-full max-w-[800px] z-10 flex flex-col gap-6 p-10 bg-[#2B2930]/30 backdrop-blur-2xl rounded-[32px] border border-white/5 shadow-2xl">
          <motion.div 
            initial={{ opacity: 0 }} animate={{ opacity: 1 }}
            className="flex items-center gap-4 border-b border-white/10 pb-6"
          >
            <DeviceHub className="text-[#D0BCFF]" style={{ fontSize: 40 }} />
            <div>
              <h1 className="text-[24px] font-semibold tracking-wide text-[#E6E1E5]">Synesthesia Matrix Initialization</h1>
              <p className="text-[16px] text-[#CAC4D0] mt-1">Cross-correlating Multi-Band Sensor Array...</p>
            </div>
          </motion.div>
          
          <div className="flex flex-col gap-3 min-h-[300px] justify-end">
            <AnimatePresence>
              {bootLogs.map((log, i) => (
                <motion.div
                  key={i}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  className={`text-[16px] font-medium tracking-wide ${log.includes('[OK]') ? 'text-[#80D8E1]' : 'text-[#D0BCFF]'}`}
                >
                  <span className="opacity-50 mr-4">sys &gt;</span>
                  {log}
                </motion.div>
              ))}
            </AnimatePresence>
            {bootLogs.length > 0 && bootLogs.length < 13 && (
              <motion.div 
                animate={{ opacity: [1, 0, 1] }} 
                transition={{ repeat: Infinity, duration: 1 }}
                className="w-3 h-5 bg-[#D0BCFF] mt-2"
              />
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div 
      className="min-h-screen flex items-center justify-center p-4 md:p-8 font-inter text-[#E6E1E5] relative overflow-hidden selection:bg-[#D0BCFF]/30 bg-transparent"
      style={{
         backgroundImage: `url('https://images.unsplash.com/photo-1557672172-298e090bd0f1?crop=entropy&cs=tinysrgb&fit=max&fm=jpg&ixid=M3w3Nzg4Nzd8MHwxfHNlYXJjaHw3fHxhYnN0cmFjdCUyMGRhcmt8ZW58MXx8fHwxNzcyOTI0NTY1fDA&ixlib=rb-4.1.0&q=80&w=1920')`,
         backgroundSize: 'cover',
         backgroundPosition: 'center',
      }}
    >
      {/* Root Window Frame - 7" Tablet Golden Ratio Scale */}
      <motion.div 
        initial={{ opacity: 0, scale: 0.98 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.5, ease: "easeOut" }}
        className="w-full max-w-[1100px] aspect-[1.618/1] min-h-[680px] bg-[#141218]/60 backdrop-blur-[40px] border border-white/10 rounded-[16px] shadow-[0_24px_80px_rgba(0,0,0,0.6)] flex flex-col relative z-10 overflow-hidden"
      >
        {/* Windows 11 Style Title Bar */}
        <div className="h-[48px] border-b border-white/5 flex items-center justify-between px-4 bg-transparent select-none">
           <div className="flex items-center gap-3">
             <DeviceHub className="text-[#D0BCFF]" style={{ fontSize: 20 }} />
             <span className="text-[14px] font-medium text-[#E6E1E5]">
               Synesthesia Multispectral Matrix
             </span>
           </div>
           
           <div className="flex items-center gap-6 text-[#CAC4D0]">
             <Minimize className="hover:text-white cursor-pointer" style={{ fontSize: 20 }} />
             <CropSquare className="hover:text-white cursor-pointer" style={{ fontSize: 18 }} />
             <Close className="hover:text-[#FFB4AB] cursor-pointer" style={{ fontSize: 22 }} />
           </div>
        </div>

        {/* Content Layout */}
        <div className="flex flex-1 overflow-hidden">
          
          {/* MD3 Navigation Rail */}
          <div className="w-[88px] flex flex-col items-center py-6 gap-8 bg-[#2B2930]/20 border-r border-white/5 z-10 shrink-0">
            <Menu className="text-[#E6E1E5] hover:text-[#D0BCFF] cursor-pointer transition-colors mb-2" style={{ fontSize: 28 }} />
            
            {[
              { id: 'grid', icon: GridView },
              { id: 'rf', icon: Sensors },
              { id: 'video', icon: Camera },
              { id: 'settings', icon: Settings }
            ].map(item => (
              <button 
                key={item.id}
                onClick={() => setActiveTab(item.id)}
                className={`w-[56px] h-[56px] rounded-full flex items-center justify-center transition-all ${activeTab === item.id ? 'bg-[#4F378B] text-[#EADDFF]' : 'text-[#CAC4D0] hover:bg-[#4A4458]/40 hover:text-[#E6E1E5]'}`}
              >
                <item.icon style={{ fontSize: 28 }} />
              </button>
            ))}
          </div>

          {/* Main Dashboard Area */}
          <div className="flex-1 p-5 overflow-y-auto scrollbar-none relative flex flex-col gap-4">
             
             {/* Diagnostics Overlay */}
             <AnimatePresence>
                {bootState === 'testing' && (
                  <motion.div 
                    initial={{ opacity: 0, backdropFilter: 'blur(0px)' }}
                    animate={{ opacity: 1, backdropFilter: 'blur(20px)' }}
                    exit={{ opacity: 0 }}
                    className="absolute inset-0 z-50 bg-[#141218]/40 flex items-center justify-center p-8 rounded-[16px]"
                  >
                    <div className="w-full max-w-[600px] bg-[#2B2930]/90 backdrop-blur-3xl border border-white/10 rounded-[32px] p-8 shadow-2xl">
                      <div className="flex items-center justify-between mb-8">
                        <div className="flex items-center gap-4">
                          <Refresh className="text-[#80D8E1] animate-spin" style={{ fontSize: 28 }} />
                          <h2 className="text-[20px] font-semibold tracking-wide text-[#E6E1E5]">Hardware Diagnostics</h2>
                        </div>
                        {testProgress >= 100 && (
                          <button 
                            onClick={() => setBootState('ready')} 
                            className="text-[16px] font-medium bg-[#4F378B] text-[#EADDFF] px-6 py-2.5 rounded-full hover:bg-[#D0BCFF] hover:text-[#381E72] transition-colors"
                          >
                            Close
                          </button>
                        )}
                      </div>

                      {/* Progress Bar */}
                      <div className="h-[8px] w-full bg-[#4A4458]/50 rounded-full overflow-hidden mb-6">
                        <motion.div 
                          className="h-full bg-[#80D8E1] rounded-full"
                          initial={{ width: '0%' }}
                          animate={{ width: `${testProgress}%` }}
                        />
                      </div>

                      {/* Results List */}
                      <div className="space-y-2 max-h-[300px] overflow-y-auto pr-2">
                        <AnimatePresence>
                          {testResults.map((result) => (
                            <motion.div 
                              key={result.id}
                              initial={{ opacity: 0, x: -20 }}
                              animate={{ opacity: 1, x: 0 }}
                              className="flex items-center justify-between p-3 rounded-[16px] bg-[#1D1B20]/40 border border-white/5"
                            >
                              <div className="flex items-center gap-4">
                                {result.status === 'PASS' ? (
                                  <CheckCircle className="text-[#80D8E1]" style={{ fontSize: 20 }} />
                                ) : (
                                  <Warning className="text-[#FFB4AB]" style={{ fontSize: 20 }} />
                                )}
                                <div>
                                   <div className="text-[14px] font-medium text-[#E6E1E5]">{result.name}</div>
                                   <div className="text-[12px] text-[#CAC4D0]">{result.addr}</div>
                                </div>
                              </div>
                              <div className="flex items-center gap-6">
                                <span className="text-[14px] text-[#CAC4D0]">{result.latency}</span>
                                <span className={`text-[12px] font-bold px-3 py-1 rounded-full ${result.status === 'PASS' ? 'bg-[#004F58] text-[#80D8E1]' : 'bg-[#93000A] text-[#FFB4AB]'}`}>
                                  {result.status}
                                </span>
                              </div>
                            </motion.div>
                          ))}
                        </AnimatePresence>
                      </div>
                    </div>
                  </motion.div>
                )}
             </AnimatePresence>

             {/* Top Status Row */}
             <div className="flex items-center justify-between px-1">
               <div>
                 <h1 className="text-[26px] font-semibold text-[#E6E1E5] tracking-tight">Array Overview</h1>
                 <p className="text-[14px] text-[#CAC4D0] mt-1 font-medium tracking-wide">Sync: Pico 2 Master Clock (PPS) • DC to 300 THz Gapless</p>
               </div>
               <button 
                 onClick={runDiagnostics}
                 className="flex items-center gap-3 bg-[#4F378B] hover:bg-[#D0BCFF] hover:text-[#381E72] text-[#EADDFF] px-5 py-2.5 rounded-full text-[14px] font-medium tracking-wide transition-colors shadow-lg"
               >
                 <Refresh style={{ fontSize: 20 }} />
                 Run Diagnostics
               </button>
             </div>

             <div className="grid grid-cols-1 lg:grid-cols-12 gap-5 flex-1 h-full min-h-0">
                
                {/* RF/SDR Card (Left Col - Span 8) */}
                <MD3Card className="lg:col-span-7 flex flex-col gap-4 h-full">
                  <div className="flex justify-between items-start">
                    <div>
                      <h3 className="text-[#CAC4D0] text-[16px] font-medium mb-1 flex items-center gap-3">
                        <Sensors className="text-[#D0BCFF]" style={{ fontSize: 20 }} />
                        RF & SDR Array (10 kHz – 6 GHz)
                      </h3>
                      <div className="flex items-baseline gap-2 mt-2">
                        <span className="text-[32px] font-light text-[#E6E1E5] tracking-tight leading-none">PlutoSDR+</span>
                        <span className="text-[16px] text-[#80D8E1] font-medium bg-[#004F58]/50 px-2 py-0.5 rounded">2TX / 2RX</span>
                      </div>
                      <p className="text-[12px] text-[#CAC4D0] mt-1">Bistatic MIMO Radar | ~12mm Baseline | TDOA Phase Interferometry</p>
                    </div>
                    <div className="bg-[#4F378B]/30 border border-[#D0BCFF]/30 px-3 py-1.5 rounded-full flex flex-col items-end">
                      <span className="text-[12px] text-[#CAC4D0]">RTL-SDR + Youloop</span>
                      <span className="text-[14px] font-medium text-[#D0BCFF]">Raw IQ / Null Axis</span>
                    </div>
                  </div>

                  {/* Main Spectrum Visualization */}
                  <div className="flex-1 w-full relative rounded-[16px] bg-[#1D1B20]/50 p-3 border border-white/5 overflow-hidden min-h-[120px]">
                    <ResponsiveContainer width="100%" height="100%">
                      <BarChart data={spectrumData}>
                        <Bar dataKey="value" isAnimationActive={false} radius={[4, 4, 0, 0]}>
                          {spectrumData.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={getSpectrumColor(entry.value)} />
                          ))}
                        </Bar>
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                </MD3Card>

                {/* Right Column Top Controls (Pico 2 Clock) */}
                <div className="lg:col-span-5 flex flex-col gap-5 h-full">
                  <MD3Card className="flex flex-col gap-4 bg-[#4F378B]/20 border-transparent">
                    <div className="flex items-center gap-3 border-b border-white/10 pb-3">
                       <Memory className="text-[#D0BCFF]" />
                       <div>
                         <h4 className="text-[14px] font-semibold text-[#E6E1E5]">Master Clock (PPS)</h4>
                         <p className="text-[12px] text-[#CAC4D0]">Pico 2 (RP2350) PIO (DC - 75 MHz)</p>
                       </div>
                    </div>

                    <MD3Switch 
                      label="PPS Sync Output" 
                      sublabel="UWB impulse TX & IR LED driver active"
                      checked={clockSync} 
                      onChange={() => setClockSync(!clockSync)} 
                    />
                    
                    <div className="flex flex-col gap-2 mt-2">
                      <span className="text-[14px] font-medium text-[#CAC4D0]">Interferometry Mode</span>
                      <div className="flex bg-[#1D1B20]/60 rounded-full p-1">
                        {['Bistatic', 'Monostatic', 'Passive'].map(m => (
                          <button 
                            key={m}
                            onClick={() => setMode(m)}
                            className={`flex-1 text-[12px] font-medium py-1.5 rounded-full transition-all ${mode === m ? 'bg-[#D0BCFF] text-[#381E72] shadow-sm' : 'text-[#CAC4D0] hover:bg-white/5'}`}
                          >
                            {m}
                          </button>
                        ))}
                      </div>
                    </div>
                  </MD3Card>
                </div>

                {/* Magnetometer / Acoustic (Bottom row left) */}
                <MD3Card className="lg:col-span-5 flex flex-col gap-4 justify-between h-[180px]">
                  <div>
                     <h3 className="text-[#CAC4D0] text-[16px] font-medium flex items-center gap-3">
                       <GraphicEq className="text-[#80D8E1]" style={{ fontSize: 20 }} />
                       Magnetometer & Acoustic
                     </h3>
                     <p className="text-[12px] text-[#CAC4D0] mt-1 ml-8">1 Hz – 90 kHz • Differential & Raw Audio</p>
                  </div>
                  
                  <div className="flex flex-col gap-3">
                    {/* Line In (Tel Coil) */}
                    <div className="flex items-center justify-between bg-[#1D1B20]/40 rounded-[12px] px-3 py-2 border border-white/5">
                      <div className="flex flex-col">
                        <span className="text-[14px] font-medium text-[#E6E1E5]">Telephone Coil (Line-in)</span>
                        <span className="text-[11px] text-[#CAC4D0]">60Hz Powerline / AC Hash Detector</span>
                      </div>
                      <span className="text-[12px] font-medium text-[#80D8E1]">Active</span>
                    </div>

                    {/* C925e Stereo */}
                    <div className="flex items-center gap-3 pl-1">
                      <span className="text-[12px] text-[#CAC4D0] font-medium w-12">C925e</span>
                      <div className="flex-1 h-[12px] flex gap-1">
                        {Array.from({length: 16}).map((_, i) => {
                          const threshold = (i / 16) * 100;
                          const isActive = threshold < audioLevel;
                          let color = 'bg-[#4A4458]/40';
                          if (isActive) {
                            if (i > 13) color = 'bg-[#FFB4AB]';
                            else if (i > 10) color = 'bg-[#D0BCFF]';
                            else color = 'bg-[#80D8E1]';
                          }
                          return (
                            <div key={i} className={`flex-1 rounded-full transition-colors duration-100 ${color}`} />
                          );
                        })}
                      </div>
                    </div>
                  </div>
                </MD3Card>

                {/* Video/Vision/Pose (Bottom row right) */}
                <MD3Card className="lg:col-span-7 flex flex-col gap-3 justify-between h-[180px]">
                  <div className="flex items-center justify-between">
                    <div>
                      <h3 className="text-[#CAC4D0] text-[16px] font-medium flex items-center gap-3">
                        <Camera className="text-[#FFB4AB]" style={{ fontSize: 20 }} />
                        Vision, Depth & Pose (~300 THz)
                      </h3>
                      <p className="text-[12px] text-[#CAC4D0] mt-1 ml-8">OV9281 Dual Stereo | C925e Visual Mic | IR Emitters</p>
                    </div>
                    <div className="flex flex-col items-end">
                      <span className="text-[16px] font-semibold text-[#E6E1E5]">2560×800</span>
                      <span className="text-[12px] text-[#FFB4AB] font-medium">120 FPS Global Shutter</span>
                    </div>
                  </div>

                  <div className="flex gap-3 h-[80px]">
                     {/* Pose Estimation / Stereo Visual Mockup */}
                     <div className="flex-1 bg-[#1D1B20]/80 rounded-[12px] border border-white/5 overflow-hidden flex items-center justify-center relative p-2">
                        {/* Fake Stereo Depth Map Background */}
                        <div className="absolute inset-0 opacity-20" style={{ background: 'linear-gradient(45deg, #381E72, #004F58)' }} />
                        
                        {/* Pose Estimation Stick Figure Overlay */}
                        <svg className="w-full h-full relative z-10" viewBox="0 0 100 50" preserveAspectRatio="xMidYMid meet">
                           {/* Shoulders */}
                           <line x1="40" y1="20" x2="60" y2="20" stroke="#80D8E1" strokeWidth="1.5" strokeLinecap="round" />
                           {/* Spine */}
                           <line x1="50" y1="20" x2="50" y2="40" stroke="#80D8E1" strokeWidth="1.5" strokeLinecap="round" />
                           {/* Arms */}
                           <line x1="40" y1="20" x2="35" y2="35" stroke="#D0BCFF" strokeWidth="1.5" strokeLinecap="round" />
                           <line x1="60" y1="20" x2="65" y2="35" stroke="#D0BCFF" strokeWidth="1.5" strokeLinecap="round" />
                           {/* Head */}
                           <circle cx="50" cy="10" r="5" fill="none" stroke="#FFB4AB" strokeWidth="1.5" />
                           {/* Joints */}
                           <circle cx="40" cy="20" r="1.5" fill="#E6E1E5" />
                           <circle cx="60" cy="20" r="1.5" fill="#E6E1E5" />
                           <circle cx="50" cy="20" r="1.5" fill="#E6E1E5" />
                           <circle cx="35" cy="35" r="1.5" fill="#E6E1E5" />
                           <circle cx="65" cy="35" r="1.5" fill="#E6E1E5" />
                        </svg>
                        <div className="absolute bottom-1 right-2 text-[8px] text-[#CAC4D0]">Structured Light Depth Active</div>
                     </div>

                     {/* Visual Mic Feed Mockup */}
                     <div className="w-[140px] bg-[#1D1B20]/50 rounded-[12px] border border-white/5 overflow-hidden flex flex-col justify-end p-1 relative">
                       <span className="absolute top-1 left-2 text-[9px] text-[#CAC4D0] z-10">C925e Visual Mic</span>
                       <ResponsiveContainer width="100%" height="60%">
                         <LineChart data={waveData}>
                           <Line type="monotone" dataKey="val" stroke="#FFB4AB" strokeWidth={1.5} dot={false} isAnimationActive={false} />
                         </LineChart>
                       </ResponsiveContainer>
                     </div>
                  </div>
                </MD3Card>

             </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
