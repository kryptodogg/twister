import React, { createContext, useContext, useEffect, useState } from 'react';
import { getDeviceStates } from '../invoke/devices';

const DeviceContext = createContext<any>(null);

export function DeviceProvider({ children }: { children: React.ReactNode }) {
  const [devices, setDevices] = useState<any>({});

  useEffect(() => {
    getDeviceStates().then((states: any) => {
      const map = Object.fromEntries(states.map((d: any) => [d.id, d]));
      setDevices(map);
    });

    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    if (isTauri) {
        import('@tauri-apps/api/event').then(({ listen }) => {
            listen('device_state_changed', (event: any) => {
                setDevices((prev: any) => ({
                    ...prev,
                    [event.payload.id]: event.payload,
                }));
            });
        });
    }
  }, []);

  return (
    <DeviceContext.Provider value={devices}>
      {children}
    </DeviceContext.Provider>
  );
}

export const useDevices = () => useContext(DeviceContext);
export const useDevice = (id: string) => {
    const devices = useContext(DeviceContext);
    return devices ? devices[id] : null;
};
