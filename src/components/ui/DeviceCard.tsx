import { useState } from 'react';
import { popOutDevice } from '../../invoke/devices';
import { useDevice } from '../../context/DeviceContext';
import { StatusChip } from './StatusChip';

interface DeviceCardProps {
  deviceId: string;
  title: string;
  children: React.ReactNode;
  accent?: string;
}

export function DeviceCard({ deviceId, title, children, accent }: DeviceCardProps) {
  const [poppedOut, setPoppedOut] = useState(false);
  const device = useDevice(deviceId);

  const handlePopOut = async () => {
    await popOutDevice(deviceId);
    setPoppedOut(true);
  };

  const status = device?.status || 'disconnected';

  if (poppedOut) {
    return (
      <md-elevated-card style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}>
        <div className="flex flex-col items-center justify-center p-8 min-h-[200px] text-on-surface/50 gap-4">
          <span className="material-symbols-outlined text-4xl text-br-teal">open_in_new</span>
          <p className="text-sm font-medium">{title} is open in a separate window</p>
          <md-outlined-button onClick={() => setPoppedOut(false)}>
            BRING BACK
          </md-outlined-button>
        </div>
      </md-elevated-card>
    );
  }

  return (
    <md-elevated-card
      style={{ '--md-elevated-card-container-color': 'var(--surface-card)' } as any}
    >
      <div className="flex flex-col h-full overflow-hidden">
        <div
          className="flex items-center justify-between p-4 border-b border-white/5"
          style={accent ? { borderLeft: `4px solid var(--color-${accent})` } : {}}
        >
          <h3 className="text-sm font-bold tracking-tight text-on-surface/90 uppercase">{title}</h3>
          <div className="flex items-center gap-2">
            <StatusChip status={status} />
            <button
              onClick={handlePopOut}
              disabled={status === 'unwired'}
              className="p-1.5 rounded-full hover:bg-white/5 text-on-surface/40 hover:text-on-surface transition-colors disabled:opacity-0 no-drag"
            >
              <span className="material-symbols-outlined text-sm">open_in_new</span>
            </button>
          </div>
        </div>
        <div className="p-5 flex-1 flex flex-col gap-4">
          {children}
        </div>
      </div>
    </md-elevated-card>
  );
}
