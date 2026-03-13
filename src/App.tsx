import React from 'react';
import { UniversalTunerCard } from './components/hardware/UniversalTunerCard';
import { motion, AnimatePresence } from 'framer-motion';

export default function App() {
  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center p-8 bg-[url('/bg.png')] bg-cover bg-center overflow-hidden">
      {/* Background Glows */}
      <div className="absolute top-0 left-0 w-full h-full pointer-events-none">
        <div className="absolute top-[-10%] left-[-10%] w-[50%] h-[50%] bg-[#D0BCFF]/10 blur-[128px] rounded-full" />
        <div className="absolute bottom-[-10%] right-[-10%] w-[50%] h-[50%] bg-[#80D8E1]/10 blur-[128px] rounded-full" />
      </div>

      <AnimatePresence>
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 20 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          transition={{ duration: 0.8, ease: "easeOut" }}
          className="z-10"
        >
          <UniversalTunerCard />
        </motion.div>
      </AnimatePresence>

      <nav className="fixed bottom-8 flex items-center gap-4 px-6 py-3 glass-card bg-black/40">
        <div className="w-10 h-10 rounded-full bg-primary/20 flex items-center justify-center text-primary-light">H</div>
        <div className="text-white/60 text-sm font-medium">Hardware Applet</div>
        <div className="w-[1px] h-4 bg-white/20" />
        <div className="text-white/40 text-xs">[UNWIRED] - Dashboard</div>
      </nav>
    </div>
  );
}
