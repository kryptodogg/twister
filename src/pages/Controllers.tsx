import { useState, useEffect } from 'react';
import { listControllers, getControllerState } from '../invoke/controllers';
import { MdSlider } from '../components/md3/Slider';
import { MdFilledTonalButton, MdOutlinedButton } from '../components/md3/Button';
import { MdSwitch } from '../components/md3/Switch';
import { MdSelect, MdSelectOption } from '../components/md3/Select';

export function Controllers() {
  const [controllers, setControllers] = useState<any[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [state, setState] = useState<any>(null);

  useEffect(() => {
    listControllers().then(list => {
      setControllers(list);
      if (list.length > 0) setSelectedId(list[0].id);
    });
  }, []);

  useEffect(() => {
    if (!selectedId) return;
    const interval = setInterval(() => {
      getControllerState(selectedId).then(setState);
    }, 50);
    return () => clearInterval(interval);
  }, [selectedId]);

  if (controllers.length === 0) return <div className="p-8 text-on-surface/50">Searching for controllers...</div>;

  return (
    <div className="flex flex-col gap-10 max-w-7xl mx-auto pb-20">
       <div className="flex flex-col gap-1">
          <h2 className="text-2xl font-bold tracking-tight uppercase">Controller Inspection</h2>
          <p className="text-xs text-on-surface/50 uppercase font-bold tracking-wider">Used to verify button integrity and sensor function. Not mapped to application controls.</p>
       </div>

       <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
          {/* Card A - Joy-Con Pair */}
          <div className="flex flex-col border border-controller/20 rounded-2xl overflow-hidden">
            <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
               <div className="flex flex-col h-full">
                 <div className="p-4 border-b border-white/5 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                       <span className="material-symbols-outlined text-controller">sports_esports</span>
                       <h3 className="text-sm font-bold uppercase text-on-surface/90">Nintendo Switch Joy-Con Pair</h3>
                    </div>
                    <div className="flex gap-2">
                       <span className="text-[9px] px-1.5 py-0.5 rounded bg-br-green/10 text-br-green font-bold">L: CONNECTED</span>
                       <span className="text-[9px] px-1.5 py-0.5 rounded bg-br-green/10 text-br-green font-bold">R: CONNECTED</span>
                    </div>
                 </div>

                 <div className="p-6 flex flex-col gap-8">
                    <div className="grid grid-cols-2 gap-8">
                       {/* Left Joy-Con */}
                       <div className="flex flex-col gap-5">
                          <div className="flex flex-col gap-1">
                            <div className="flex justify-between text-[10px] font-bold text-on-surface/50 uppercase">
                              <span>Joy-Con (L) Battery</span>
                              <span>75%</span>
                            </div>
                            <md-linear-progress value={0.75} style={{ '--md-linear-progress-active-indicator-color': 'var(--br-purple)' } as any}></md-linear-progress>
                          </div>

                          <div className="grid grid-cols-1 gap-2">
                             <div className="flex justify-between text-[10px] font-mono">
                                <span className="text-on-surface/40 uppercase">Accel</span>
                                <span className="text-controller font-bold">X: 0.1 Y: 9.8 Z: 0.2</span>
                             </div>
                             <div className="flex justify-between text-[10px] font-mono">
                                <span className="text-on-surface/40 uppercase">Gyro</span>
                                <span className="text-controller font-bold">X: 0.0 Y: 0.0 Z: 0.0</span>
                             </div>
                          </div>

                          <div className="grid grid-cols-4 gap-1">
                             {['SL', 'SR', 'L', 'ZL', '-', 'UP', 'DN', 'LT', 'RT', 'CAP', 'L3'].map(btn => (
                               <div key={btn} className="aspect-square rounded bg-white/5 border border-white/5 flex items-center justify-center text-[8px] font-bold text-on-surface/30">
                                  {btn}
                               </div>
                             ))}
                          </div>
                       </div>

                       {/* Right Joy-Con */}
                       <div className="flex flex-col gap-5">
                          <div className="flex flex-col gap-1">
                            <div className="flex justify-between text-[10px] font-bold text-on-surface/50 uppercase">
                              <span>Joy-Con (R) Battery</span>
                              <span>82%</span>
                            </div>
                            <md-linear-progress value={0.82} style={{ '--md-linear-progress-active-indicator-color': 'var(--br-purple)' } as any}></md-linear-progress>
                          </div>

                          <div className="grid grid-cols-1 gap-2">
                             <div className="flex justify-between text-[10px] font-mono">
                                <span className="text-on-surface/40 uppercase">Accel</span>
                                <span className="text-controller font-bold">X: -0.1 Y: 9.7 Z: -0.3</span>
                             </div>
                             <div className="flex justify-between text-[10px] font-mono">
                                <span className="text-on-surface/40 uppercase">Gyro</span>
                                <span className="text-controller font-bold">X: 0.0 Y: 0.1 Z: 0.0</span>
                             </div>
                          </div>

                          <div className="grid grid-cols-4 gap-1">
                             {['SL', 'SR', 'R', 'ZR', '+', 'A', 'B', 'X', 'Y', 'HOME', 'R3'].map(btn => (
                               <div key={btn} className="aspect-square rounded bg-white/5 border border-white/5 flex items-center justify-center text-[8px] font-bold text-on-surface/30">
                                  {btn}
                               </div>
                             ))}
                          </div>

                          <MdSelect label="IR Camera Mode" value="off">
                             <MdSelectOption value="off">Off</MdSelectOption>
                             <MdSelectOption value="standard">Standard</MdSelectOption>
                             <MdSelectOption value="wide">Wide</MdSelectOption>
                          </MdSelect>
                       </div>
                    </div>

                    <div className="pt-6 border-t border-white/5 flex flex-col gap-4">
                       <div className="flex justify-between text-[10px] font-bold text-on-surface/50 uppercase">
                         <span>Rumble Intensity</span>
                         <span>50%</span>
                       </div>
                       <MdSlider min={0} max={100} value={50} />
                       <div className="flex gap-2">
                          <MdFilledTonalButton className="flex-1">TEST LEFT</MdFilledTonalButton>
                          <MdFilledTonalButton className="flex-1">TEST RIGHT</MdFilledTonalButton>
                       </div>
                    </div>
                 </div>
               </div>
            </md-elevated-card>
          </div>

          {/* Card B - DualSense */}
          <div className="flex flex-col border border-controller/20 rounded-2xl overflow-hidden">
            <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
               <div className="flex flex-col h-full">
                 <div className="p-4 border-b border-white/5 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                       <span className="material-symbols-outlined text-controller">videogame_asset</span>
                       <h3 className="text-sm font-bold uppercase text-on-surface/90">PlayStation 5 DualSense</h3>
                    </div>
                    <span className="text-[9px] px-1.5 py-0.5 rounded bg-br-green/10 text-br-green font-bold">CONNECTED (USB)</span>
                 </div>

                 <div className="p-6 flex flex-col gap-8">
                    <div className="grid grid-cols-2 gap-8">
                       <div className="flex flex-col gap-6">
                          <div className="grid grid-cols-4 gap-1">
                             {['L1', 'R1', 'L2', 'R2', 'SHR', 'OPT', 'PS', 'PAD', 'UP', 'DN', 'L', 'R', 'SQ', 'TR', 'CI', 'CR'].map(btn => (
                               <div key={btn} className="aspect-square rounded bg-white/5 border border-white/5 flex items-center justify-center text-[8px] font-bold text-on-surface/30">
                                  {btn}
                               </div>
                             ))}
                          </div>

                          <div className="flex flex-col gap-4">
                             <div className="p-3 rounded bg-white/5 border border-white/5">
                                <span className="text-[9px] font-bold text-on-surface/40 uppercase block mb-2">Touchpad</span>
                                <div className="w-full aspect-[2/1] bg-black/20 rounded relative border border-white/5">
                                   <div className="absolute w-2 h-2 rounded-full bg-controller/60 left-[30%] top-[40%]"></div>
                                   <div className="absolute w-2 h-2 rounded-full bg-controller/60 left-[60%] top-[55%]"></div>
                                </div>
                             </div>
                          </div>
                       </div>

                       <div className="flex flex-col gap-6">
                          <div className="flex flex-col gap-3">
                             <span className="text-[9px] font-bold text-on-surface/40 uppercase">Adaptive Triggers</span>
                             <div className="flex flex-col gap-2">
                                <div className="flex justify-between items-center text-[10px]">
                                   <span className="text-on-surface/60">L2 PROFILE</span>
                                   <span className="px-1.5 py-0.5 rounded bg-tx/10 text-tx font-bold uppercase">Weapon</span>
                                </div>
                                <div className="flex justify-between items-center text-[10px]">
                                   <span className="text-on-surface/60">R2 PROFILE</span>
                                   <span className="px-1.5 py-0.5 rounded bg-tx/10 text-tx font-bold uppercase">Machine</span>
                                </div>
                             </div>
                          </div>

                          <div className="flex flex-col gap-4">
                             <div className="flex items-center justify-between">
                                <span className="text-xs font-medium">Mute LED</span>
                                <MdSwitch />
                             </div>
                             <div className="flex flex-col gap-2">
                                <span className="text-[9px] font-bold text-on-surface/40 uppercase">Lightbar Color</span>
                                <div className="flex items-center gap-3">
                                   <div className="w-8 h-8 rounded bg-br-purple border border-white/10 shadow-lg"></div>
                                   <span className="text-[10px] font-mono text-controller uppercase">#A954BF</span>
                                </div>
                             </div>
                          </div>
                       </div>
                    </div>

                    <div className="pt-6 border-t border-white/5 flex flex-col gap-6">
                       <div className="grid grid-cols-2 gap-4">
                          <div className="flex flex-col gap-2">
                             <span className="text-[9px] font-bold text-on-surface/40 uppercase">Haptic Intensity</span>
                             <MdSlider min={0} max={100} value={80} />
                          </div>
                          <div className="flex flex-col gap-2">
                             <span className="text-[9px] font-bold text-on-surface/40 uppercase">Speaker Volume</span>
                             <MdSlider min={0} max={100} value={40} />
                          </div>
                       </div>
                       <div className="flex gap-2">
                          <MdOutlinedButton className="flex-1">TRIGGER TEST</MdOutlinedButton>
                          <MdFilledTonalButton className="flex-1">FIRE HAPTIC</MdFilledTonalButton>
                       </div>
                    </div>
                 </div>
               </div>
            </md-elevated-card>
          </div>
       </div>
    </div>
  );
}
