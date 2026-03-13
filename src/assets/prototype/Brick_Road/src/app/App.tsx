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
  PlayArrow,
  Refresh,
  CheckCircle,
  Warning,
  PowerSettingsNew
} from '@mui/icons-material';
import { BarChart, Bar, Cell, ResponsiveContainer, LineChart, Line, XAxis } from 'recharts';

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

const MD3Switch = ({ checked, onChange, label }: { checked: boolean, onChange: () => void, label: string }) => (
  <div className="flex items-center justify-between gap-4 w-full cursor-pointer" onClick={onChange}>
    <span className="text-[16px] text-[#E6E1E5] font-medium tracking-wide">{label}</span>
    <div className={`w-[52px] h-[32px] rounded-full p-[2px] transition-colors relative border ${checked ? 'bg-[#D0BCFF] border-[#D0BCFF]' : 'bg-[#36343B] border-[#938F99]'}`}>
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

const MD3Slider = ({ value, onChange, label, unit }: { value: number, onChange: (v: number) => void, label: string, unit?: string }) => (
  <div className="flex flex-col gap-2 w-full">
    <div className="flex justify-between items-center">
      <span className="text-[16px] text-[#E6E1E5] font-medium">{label}</span>
      <span className="text-[16px] text-[#D0BCFF] font-medium">{value}{unit}</span>
    </div>
    <div className="relative w-full h-[40px] flex items-center cursor-pointer group">
      {/* Track */}
      <div className="absolute w-full h-[16px] rounded-full bg-[#4A4458]/50 overflow-hidden">
        <div 
          className="h-full bg-[#D0BCFF] rounded-full transition-all" 
          style={{ width: `${value}%` }} 
        />
      </div>
      {/* Thumb */}
      <input 
        type="range" 
        min="0" max="100" 
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="absolute w-full h-full opacity-0 cursor-pointer z-10" 
      />
      <div 
        className="absolute h-[4px] w-[2px] bg-[#381E72] pointer-events-none transition-all group-active:scale-x-150"
        style={{ left: `calc(${value}% - 1px)` }}
      />
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
  const [anomalySens, setAnomalySens] = useState(65);
  const [gainLevel, setGainLevel] = useState(42);
  const [mode, setMode] = useState('Auto');
  const [beaconOn, setBeaconOn] = useState(true);
  
  const [testResults, setTestResults] = useState<any[]>([]);
  const [testProgress, setTestProgress] = useState(0);

  const detectedHardware = [
    { id: 'qpc', name: 'QPC Sync Module', addr: '0x00A1' },
    { id: 'sdr', name: 'Pluto+ SDR Array', addr: 'USB:0456' },
    { id: 'cam', name: 'Logitech C925e (RAW)', addr: 'USB:082D' },
    { id: 'aud', name: '8-Ch Phased Audio', addr: 'PCIe:00:1F' },
    { id: 'rtl', name: 'RTL-SDR Tuner', addr: 'USB:0BDA' }
  ];

  // Boot Sequence Simulation
  useEffect(() => {
    if (bootState !== 'booting') return;

    const sequence = [
      "Initializing Synesthesia Control...",
      "Loading FieldParticle Engine v4.2...",
      "Scanning hardware buses...",
      ...detectedHardware.map(hw => `[OK] Found ${hw.name} at ${hw.addr}`),
      "Hardware auto-population complete."
    ];

    let delay = 0;
    sequence.forEach((log, index) => {
      delay += 500 + Math.random() * 300;
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
      
      if (progress % 20 === 0 && hwIndex < detectedHardware.length) {
         setTestResults(prev => {
            const hw = detectedHardware[prev.length];
            if (!hw) return prev;
            return [...prev, { ...hw, status: Math.random() > 0.9 ? 'WARN' : 'PASS', latency: `${(Math.random() * 5 + 0.1).toFixed(1)}ms` }];
         });
      }

      if (progress >= 100) clearInterval(testInterval);
    }, 150);
  };

  // --- RENDERERS ---

  if (bootState === 'booting') {
    return (
      <div className="min-h-screen bg-[#141218] flex flex-col items-center justify-center p-8 font-inter text-[#E6E1E5] relative overflow-hidden selection:bg-[#D0BCFF]/30">
        <div className="w-full max-w-[800px] z-10 flex flex-col gap-6 p-10 bg-[#2B2930]/30 backdrop-blur-2xl rounded-[32px] border border-white/5 shadow-2xl">
          <motion.div 
            initial={{ opacity: 0 }} animate={{ opacity: 1 }}
            className="flex items-center gap-4 border-b border-white/10 pb-6"
          >
            <PowerSettingsNew className="text-[#D0BCFF]" style={{ fontSize: 40 }} />
            <div>
              <h1 className="text-[24px] font-semibold tracking-wide text-[#E6E1E5]">Synesthesia Initialization</h1>
              <p className="text-[16px] text-[#CAC4D0] mt-1">Awaiting Hardware Handshake...</p>
            </div>
          </motion.div>
          
          <div className="flex flex-col gap-3 min-h-[250px] justify-end">
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
            {bootLogs.length > 0 && bootLogs.length < 9 && (
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

  // Golden Ratio Container: roughly 1.618:1 aspect ratio (e.g., 1100px x 680px)
  return (
    <div 
      className="min-h-screen flex items-center justify-center p-4 md:p-8 font-inter text-[#E6E1E5] relative overflow-hidden selection:bg-[#D0BCFF]/30 bg-transparent"
      style={{
         // Simulate Windows 11 Desktop behind the app window
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
             <GraphicEq className="text-[#D0BCFF]" style={{ fontSize: 20 }} />
             <span className="text-[16px] font-medium text-[#E6E1E5]">
               Synesthesia Console
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
          <div className="w-[88px] flex flex-col items-center py-6 gap-8 bg-[#2B2930]/20 border-r border-white/5 z-10">
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
          <div className="flex-1 p-6 overflow-y-auto scrollbar-none relative flex flex-col gap-6">
             
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
                            Done
                          </button>
                        )}
                      </div>

                      {/* Progress Bar */}
                      <div className="h-[8px] w-full bg-[#4A4458]/50 rounded-full overflow-hidden mb-8">
                        <motion.div 
                          className="h-full bg-[#80D8E1] rounded-full"
                          initial={{ width: '0%' }}
                          animate={{ width: `${testProgress}%` }}
                        />
                      </div>

                      {/* Results List */}
                      <div className="space-y-3">
                        <AnimatePresence>
                          {testResults.map((result) => (
                            <motion.div 
                              key={result.id}
                              initial={{ opacity: 0, x: -20 }}
                              animate={{ opacity: 1, x: 0 }}
                              className="flex items-center justify-between p-4 rounded-[16px] bg-[#1D1B20]/40 border border-white/5"
                            >
                              <div className="flex items-center gap-4">
                                {result.status === 'PASS' ? (
                                  <CheckCircle className="text-[#80D8E1]" style={{ fontSize: 24 }} />
                                ) : (
                                  <Warning className="text-[#FFB4AB]" style={{ fontSize: 24 }} />
                                )}
                                <span className="text-[16px] font-medium text-[#E6E1E5]">{result.name}</span>
                              </div>
                              <div className="flex items-center gap-6">
                                <span className="text-[16px] text-[#CAC4D0]">{result.latency}</span>
                                <span className={`text-[16px] font-bold px-4 py-1.5 rounded-full ${result.status === 'PASS' ? 'bg-[#004F58] text-[#80D8E1]' : 'bg-[#93000A] text-[#FFB4AB]'}`}>
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
             <div className="flex items-center justify-between px-2">
               <div>
                 <h1 className="text-[28px] font-semibold text-[#E6E1E5] tracking-tight">Status Overview</h1>
                 <p className="text-[16px] text-[#CAC4D0] mt-1 font-medium tracking-wide">System Link Established • 5 Devices</p>
               </div>
               <button 
                 onClick={runDiagnostics}
                 className="flex items-center gap-3 bg-[#4F378B] hover:bg-[#D0BCFF] hover:text-[#381E72] text-[#EADDFF] px-6 py-3 rounded-full text-[16px] font-medium tracking-wide transition-colors shadow-lg"
               >
                 <Refresh style={{ fontSize: 22 }} />
                 Run Diagnostics
               </button>
             </div>

             <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 flex-1 h-full">
                
                {/* RF/SDR Card */}
                <MD3Card className="lg:col-span-8 flex flex-col gap-6 h-full">
                  <div className="flex justify-between items-start">
                    <div>
                      <h3 className="text-[#CAC4D0] text-[16px] font-medium mb-2 flex items-center gap-3">
                        <Sensors className="text-[#D0BCFF]" style={{ fontSize: 22 }} />
                        RF / SDR Array (Pluto+)
                      </h3>
                      <div className="flex items-baseline gap-2">
                        <span className="text-[40px] font-light text-[#E6E1E5] tracking-tight leading-none">2.450</span>
                        <span className="text-[20px] text-[#CAC4D0] font-medium">GHz</span>
                      </div>
                    </div>
                    <div className="bg-[#4F378B]/30 border border-[#D0BCFF]/30 px-4 py-2 rounded-full">
                      <span className="text-[16px] font-medium text-[#D0BCFF]">BW: 1.40 MHz</span>
                    </div>
                  </div>

                  {/* Main Spectrum Visualization */}
                  <div className="flex-1 w-full relative rounded-[20px] bg-[#1D1B20]/50 p-4 border border-white/5 overflow-hidden min-h-[160px]">
                    <ResponsiveContainer width="100%" height="100%">
                      <BarChart data={spectrumData}>
                        <Bar dataKey="value" isAnimationActive={false} radius={[6, 6, 0, 0]}>
                          {spectrumData.map((entry, index) => (
                            <Cell key={`cell-${index}`} fill={getSpectrumColor(entry.value)} />
                          ))}
                        </Bar>
                      </BarChart>
                    </ResponsiveContainer>
                  </div>
                </MD3Card>

                {/* Right Column Controls */}
                <div className="lg:col-span-4 flex flex-col gap-6 h-full">
                  
                  {/* Mode & Beacon Tonal Card */}
                  <MD3Card className="flex flex-col gap-6 bg-[#4F378B]/20 border-transparent">
                    <MD3Switch 
                      label="Operational Beacon" 
                      checked={beaconOn} 
                      onChange={() => setBeaconOn(!beaconOn)} 
                    />
                    
                    <div className="flex flex-col gap-3">
                      <span className="text-[16px] font-medium text-[#CAC4D0]">Operating Mode</span>
                      <div className="flex bg-[#1D1B20]/60 rounded-full p-1.5">
                        {['Auto', 'Manual', 'Passive'].map(m => (
                          <button 
                            key={m}
                            onClick={() => setMode(m)}
                            className={`flex-1 text-[16px] font-medium py-2 rounded-full transition-all ${mode === m ? 'bg-[#D0BCFF] text-[#381E72] shadow-sm' : 'text-[#CAC4D0] hover:bg-white/5'}`}
                          >
                            {m}
                          </button>
                        ))}
                      </div>
                    </div>
                  </MD3Card>

                  {/* Settings / Sliders */}
                  <MD3Card className="flex-1 flex flex-col justify-center gap-8">
                     <MD3Slider 
                        label="Anomaly Sens." 
                        value={anomalySens} 
                        onChange={setAnomalySens} 
                        unit="%"
                     />
                     <MD3Slider 
                        label="Input Gain" 
                        value={gainLevel} 
                        onChange={setGainLevel} 
                        unit=" dB"
                     />
                  </MD3Card>
                </div>

                {/* Audio/Mic (Bottom row left) */}
                <MD3Card className="lg:col-span-6 flex flex-col gap-5 justify-between min-h-[180px]">
                  <h3 className="text-[#CAC4D0] text-[16px] font-medium flex items-center gap-3">
                    <PlayArrow className="text-[#80D8E1]" style={{ fontSize: 24 }} />
                    Audio Array (Phased)
                  </h3>
                  
                  <div className="flex flex-col gap-4">
                    <div className="flex items-center gap-4">
                      <span className="text-[16px] text-[#CAC4D0] font-medium w-8">Vol</span>
                      {/* MD3 Tonal Level Meter */}
                      <div className="flex-1 h-[20px] flex gap-1.5">
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
                    <div className="flex items-center justify-between bg-[#1D1B20]/40 rounded-full px-5 py-2.5 border border-white/5 cursor-pointer hover:bg-white/5">
                      <span className="text-[16px] font-medium text-[#E6E1E5]">Channel 1 (Primary)</span>
                      <span className="text-[16px] font-medium text-[#80D8E1]">Sync OK</span>
                    </div>
                  </div>
                </MD3Card>

                {/* Video/CMOS (Bottom row right) */}
                <MD3Card className="lg:col-span-6 flex flex-col gap-4 justify-between min-h-[180px]">
                  <div className="flex items-center justify-between">
                    <h3 className="text-[#CAC4D0] text-[16px] font-medium flex items-center gap-3">
                      <Camera className="text-[#FFB4AB]" style={{ fontSize: 22 }} />
                      Video CMOS
                    </h3>
                    <div className="flex items-baseline gap-2">
                      <span className="text-[24px] font-semibold text-[#E6E1E5]">128</span>
                      <span className="text-[16px] text-[#CAC4D0] font-medium">FPS</span>
                    </div>
                  </div>

                  <div className="h-[70px] w-full bg-[#1D1B20]/50 rounded-[16px] overflow-hidden p-2 border border-white/5 relative">
                     <ResponsiveContainer width="100%" height="100%">
                       <LineChart data={waveData}>
                         <Line type="basis" dataKey="val" stroke="#FFB4AB" strokeWidth={3} dot={false} isAnimationActive={false} />
                       </LineChart>
                     </ResponsiveContainer>
                  </div>
                </MD3Card>

             </div>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
