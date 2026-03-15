import { useState, useEffect } from 'react';
import { getGpioAssignments, saveGpioAssignments } from '../invoke/gpio';
import { MdSelect, MdSelectOption } from '../components/md3/Select';
import { MdTextField } from '../components/md3/TextField';
import { MdFilledTonalButton } from '../components/md3/Button';
import { cn } from '../app/components/ui/utils';

interface GpioPin {
  pin: number;
  function: string;
  direction: 'IN' | 'OUT' | 'ALT';
  pull: 'UP' | 'DOWN' | 'NONE';
  connected_to: string;
  active_state: 'HIGH' | 'LOW';
}

const CATEGORIES: Record<string, string> = {
  'sensor': 'bg-rx border-rx/30',
  'driver': 'bg-tx border-tx/30',
  'clock/sync': 'bg-config border-config/30',
  'neutral': 'bg-br-slate border-br-slate/30',
};

function getCategory(pin: GpioPin) {
  const func = pin.function.toLowerCase();
  if (func.includes('led') || func.includes('trigger') || (pin.direction === 'OUT' && !func.includes('pps'))) return 'driver';
  if (func.includes('receiver') || func.includes('data') || (pin.direction === 'IN' && !func.includes('pps'))) return 'sensor';
  if (func.includes('pps') || func.includes('i2c') || func.includes('spi') || func.includes('clk')) return 'clock/sync';
  return 'neutral';
}

export function GPIO() {
  const [pins, setPins] = useState<GpioPin[]>([]);
  const [editingPin, setEditingPin] = useState<GpioPin | null>(null);

  useEffect(() => {
    getGpioAssignments().then(setPins);
  }, []);

  const handleSave = async () => {
    await saveGpioAssignments(pins);
  };

  const updatePin = (patch: Partial<GpioPin>) => {
    if (!editingPin) return;
    const updated = { ...editingPin, ...patch };
    setEditingPin(updated);
    setPins(prev => prev.map(p => p.pin === updated.pin ? updated : p));
  };

  const renderPin = (pinNum: number) => {
    const pin = pins.find(p => p.pin === pinNum);
    const category = pin ? getCategory(pin) : 'neutral';
    const active = editingPin?.pin === pinNum;

    return (
      <button
        key={pinNum}
        onClick={() => setEditingPin(pin || { pin: pinNum, function: '', direction: 'IN', pull: 'NONE', connected_to: '', active_state: 'HIGH' })}
        className={cn(
          "w-8 h-8 rounded-full border-2 flex items-center justify-center transition-all hover:scale-110 no-drag",
          active ? "ring-2 ring-white scale-125 z-10" : "",
          CATEGORIES[category]
        )}
      >
        <span className="text-[10px] font-bold text-white/90">{pinNum}</span>
      </button>
    );
  };

  return (
    <div className="flex flex-col gap-8 max-w-6xl mx-auto pb-20">
      <div className="flex items-center justify-between">
         <h2 className="text-2xl font-bold tracking-tight uppercase">Raspberry Pi GPIO</h2>
         <div className="flex gap-3">
            <MdFilledTonalButton onClick={handleSave}>SAVE ASSIGNMENTS</MdFilledTonalButton>
         </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[1fr_350px] gap-8">
        <div className="flex flex-col gap-8">
          <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
             <div className="overflow-hidden">
                <table className="w-full text-left text-xs">
                   <thead className="bg-white/5 text-on-surface/50 uppercase font-bold border-b border-white/5">
                      <tr>
                         <th className="p-4">PIN (BCM)</th>
                         <th className="p-4">FUNCTION</th>
                         <th className="p-4">DIR</th>
                         <th className="p-4">PULL</th>
                         <th className="p-4">CONNECTED TO</th>
                      </tr>
                   </thead>
                   <tbody className="text-on-surface/90">
                      {pins.map(pin => (
                        <tr
                          key={pin.pin}
                          className={cn(
                            "border-b border-white/5 hover:bg-white/5 cursor-pointer transition-colors",
                            editingPin?.pin === pin.pin ? "bg-white/10" : ""
                          )}
                          onClick={() => setEditingPin(pin)}
                        >
                           <td className="p-4 font-mono font-bold text-br-teal">{pin.pin}</td>
                           <td className="p-4">{pin.function}</td>
                           <td className="p-4"><span className="px-1.5 py-0.5 rounded bg-white/5 font-mono">{pin.direction}</span></td>
                           <td className="p-4 text-on-surface/50">{pin.pull}</td>
                           <td className="p-4 italic text-on-surface/70">{pin.connected_to}</td>
                        </tr>
                      ))}
                   </tbody>
                </table>
             </div>
          </md-elevated-card>

          <div className="flex flex-wrap gap-6 p-4 rounded-xl bg-surface-card border border-white/5">
             <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-rx"></div>
                <span className="text-[10px] uppercase font-bold text-on-surface/60">Sensor Input</span>
             </div>
             <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-tx"></div>
                <span className="text-[10px] uppercase font-bold text-on-surface/60">Output / Driver</span>
             </div>
             <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-config"></div>
                <span className="text-[10px] uppercase font-bold text-on-surface/60">Clock / Sync</span>
             </div>
             <div className="flex items-center gap-2">
                <div className="w-3 h-3 rounded-full bg-br-slate"></div>
                <span className="text-[10px] uppercase font-bold text-on-surface/60">Unassigned</span>
             </div>
          </div>
        </div>

        <div className="flex flex-col gap-6">
          <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
             <div className="p-6">
                <div className="flex flex-col items-center gap-2">
                   <span className="text-[10px] font-bold text-on-surface/40 uppercase tracking-widest mb-4">40-Pin Header Map</span>
                   <div className="grid grid-cols-2 gap-x-12 gap-y-2">
                      {Array.from({ length: 20 }).map((_, i) => (
                        <div key={i} className="flex gap-2">
                           {renderPin(i * 2 + 1)}
                           {renderPin(i * 2 + 2)}
                        </div>
                      ))}
                   </div>
                </div>
             </div>
          </md-elevated-card>

          {editingPin && (
            <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
               <div className="p-6 border border-br-teal/30 rounded-xl">
                  <div className="flex flex-col gap-5">
                     <div className="flex items-center justify-between">
                       <h3 className="text-sm font-bold uppercase">Edit Pin {editingPin.pin}</h3>
                       <button onClick={() => setEditingPin(null)} className="text-on-surface/40 hover:text-white no-drag">
                         <span className="material-symbols-outlined text-sm">close</span>
                       </button>
                     </div>

                     <MdTextField
                       label="Function Name"
                       value={editingPin.function}
                       onInput={(v) => updatePin({ function: v })}
                     />

                     <MdSelect
                       label="Direction"
                       value={editingPin.direction}
                       onSelected={(v: any) => updatePin({ direction: v })}
                     >
                        <MdSelectOption value="IN">Input (IN)</MdSelectOption>
                        <MdSelectOption value="OUT">Output (OUT)</MdSelectOption>
                        <MdSelectOption value="ALT">Alternate (ALT)</MdSelectOption>
                     </MdSelect>

                     <MdSelect
                       label="Pull Resistance"
                       value={editingPin.pull}
                       onSelected={(v: any) => updatePin({ pull: v })}
                     >
                        <MdSelectOption value="NONE">None</MdSelectOption>
                        <MdSelectOption value="UP">Pull Up</MdSelectOption>
                        <MdSelectOption value="DOWN">Pull Down</MdSelectOption>
                     </MdSelect>

                     <MdTextField
                       label="Connected To"
                       value={editingPin.connected_to}
                       onInput={(v) => updatePin({ connected_to: v })}
                     />

                     <MdSelect
                       label="Active State"
                       value={editingPin.active_state}
                       onSelected={(v: any) => updatePin({ active_state: v })}
                     >
                        <MdSelectOption value="HIGH">Active HIGH</MdSelectOption>
                        <MdSelectOption value="LOW">Active LOW</MdSelectOption>
                     </MdSelect>
                  </div>
               </div>
            </md-elevated-card>
          )}
        </div>
      </div>
    </div>
  );
}
