import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { Radio, Satellite, Zap, Activity, Settings2, SlidersHorizontal } from 'lucide-react';

export const UniversalTunerCard: React.FC = () => {
  const [frequency, setFrequency] = useState("102.100");
  const [antenna, setAntenna] = useState("YouLoop");
  const [modulation, setModulation] = useState("FM");

  // Golden Ratio Scaling (1000px x 618px base)
  return (
    <div 
      className="relative w-[1000px] h-[618px] glass-card flex flex-col overflow-hidden"
      style={{
        background: 'rgba(255, 255, 255, 0.03)',
        boxShadow: '0 8px 32px 0 rgba(0, 0, 0, 0.8)',
      }}
    >
      {/* Top Bar */}
      <div className="flex items-center justify-between px-8 py-6 border-b border-white/5">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-primary/20 rounded-lg text-primary">
            <Radio size={24} />
          </div>
          <div>
            <h2 className="text-xl font-bold tracking-tight text-white/90">Universal Tuner</h2>
            <p className="text-xs text-white/40 font-mono tracking-widest uppercase">Platform: RTL-SDR [DISCONNECTED]</p>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <button className="flex items-center gap-2 px-4 py-2 bg-white/5 hover:bg-white/10 rounded-md transition-colors border border-white/10 group">
             <Settings2 size={18} className="text-white/60 group-hover:text-primary transition-colors" />
             <span className="text-xs font-semibold text-white/80">Settings</span>
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 flex gap-px bg-white/5">
        {/* Left Control Panel (1/3 Width) */}
        <div className="w-[382px] bg-[#0A0A0A]/40 p-10 flex flex-col gap-8">
          {/* Antenna Selection */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <label className="text-xs font-bold text-white/40 uppercase tracking-widest">Antenna Preset</label>
              <Zap size={14} className="text-secondary" />
            </div>
            <div className="grid grid-cols-2 gap-3">
              {["Bundled", "YouLoop"].map((a) => (
                <button
                  key={a}
                  onClick={() => setAntenna(a)}
                  className={`py-3 rounded-lg text-xs font-bold transition-all border ${
                    antenna === a 
                      ? 'bg-secondary/20 border-secondary/40 text-secondary' 
                      : 'bg-white/5 border-white/5 text-white/40 hover:border-white/20'
                  }`}
                >
                  {a}
                </button>
              ))}
            </div>
            <p className="text-[10px] text-white/30 font-mono italic">
              {antenna === "YouLoop" ? "Range: 10 kHz - 300 MHz" : "Standard Dipole Kit"}
            </p>
          </div>

          {/* Modulation */}
          <div className="space-y-4">
            <label className="text-xs font-bold text-white/40 uppercase tracking-widest">Modulation Flow</label>
            <div className="grid grid-cols-3 gap-2">
              {["AM", "FM", "USB", "LSB", "CW", "W-OFDM"].map((m) => (
                <button
                  key={m}
                  onClick={() => setModulation(m)}
                  className={`py-2 rounded-md text-[10px] font-bold transition-all border ${
                    modulation === m 
                      ? 'bg-primary/20 border-primary/40 text-primary' 
                      : 'bg-white/5 border-white/5 text-white/40 hover:border-white/20'
                  }`}
                >
                  {m}
                </button>
              ))}
            </div>
          </div>

          <div className="mt-auto p-4 bg-primary/5 rounded-xl border border-primary/10">
            <div className="flex items-center gap-3 mb-2">
              <Activity size={16} className="text-primary" />
              <span className="text-xs font-bold text-white/60">Live Signal Density</span>
            </div>
            <div className="h-1 w-full bg-white/10 rounded-full overflow-hidden">
               <motion.div 
                 initial={{ width: 0 }} 
                 animate={{ width: '45%' }} 
                 transition={{ repeat: Infinity, duration: 2, repeatType: 'reverse' }}
                 className="h-full bg-primary" 
               />
            </div>
          </div>
        </div>

        {/* Right Spectrum Area (2/3 Width) */}
        <div className="flex-1 bg-[#0A0A0A]/20 flex flex-col p-8 relative">
          <div className="flex-1 flex flex-col items-center justify-center gap-12">
            {/* Massive Frequency Readout */}
            <div className="text-center">
               <div className="flex items-end gap-3 justify-center">
                 <h1 className="text-[120px] leading-none font-black tracking-tighter text-white/90 glow-text flex">
                   {frequency}
                 </h1>
                 <span className="text-4xl font-bold text-primary mb-6">MHz</span>
               </div>
               <div className="flex items-center justify-center gap-6 mt-4">
                 <div className="px-3 py-1 bg-white/5 rounded text-[10px] font-mono text-white/60 border border-white/10">IF: 0.00 Hz</div>
                 <div className="px-3 py-1 bg-white/5 rounded text-[10px] font-mono text-white/60 border border-white/10">SR: 2.4 MSPS</div>
               </div>
            </div>

            {/* Simulated Waterfall/Spectrum - Absolute forensic integrity: NO fake signal if disconnected */}
            <div className="w-full h-48 bg-black/40 rounded-xl border border-white/5 flex flex-col items-center justify-center relative overflow-hidden group">
               <div className="absolute inset-0 opacity-20 pointer-events-none">
                  {/* Grid lines */}
                  <div className="w-full h-full grid grid-cols-12 gap-px border-white/10">
                    {[...Array(12)].map((_, i) => <div key={i} className="h-full border-r border-white/5" />)}
                  </div>
               </div>
               
               <p className="text-xs text-secondary/60 font-mono tracking-widest animate-pulse">[ NO SOURCE CONNECTED ]</p>
               <p className="text-[10px] text-white/20 mt-2">Waiting for RTL-SDR USB Packet Stream...</p>

               <div className="absolute bottom-4 right-4 group-hover:block hidden transition-all">
                  <div className="px-3 py-1 bg-secondary/10 border border-secondary/20 rounded text-[10px] text-secondary">Aether Analysis [UNWIRED]</div>
               </div>
            </div>
          </div>
        </div>
      </div>
      
      {/* Dynamic Glow Element that follows cursor (subtle) */}
      <div className="absolute bottom-0 right-0 w-64 h-64 bg-primary/5 blur-[80px] pointer-events-none rounded-full" />
    </div>
  );
};
