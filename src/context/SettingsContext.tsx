import React, { createContext, useContext, useEffect, useState } from 'react';
import { getSettings, saveSettings } from '../invoke/settings';

const SettingsContext = createContext<any>(null);

export function SettingsProvider({ children }: { children: React.ReactNode }) {
  const [settings, setSettings] = useState<any>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    getSettings().then(setSettings);
  }, []);

  const updateDevice = (deviceId: string, patch: any) => {
    setSettings((prev: any) => ({
      ...prev,
      devices: {
        ...prev.devices,
        [deviceId]: { ...prev.devices[deviceId], ...patch }
      }
    }));
    setDirty(true);
  };

  const updateGpio = (pin: number, patch: any) => {
    setSettings((prev: any) => ({
      ...prev,
      gpio: prev.gpio.map((p: any) => p.pin === pin ? { ...p, ...patch } : p)
    }));
    setDirty(true);
  };

  const save = async () => {
    await saveSettings(settings);
    setDirty(false);
  };

  return (
    <SettingsContext.Provider value={{ settings, updateDevice, updateGpio, save, dirty }}>
      {children}
    </SettingsContext.Provider>
  );
}

export const useSettings = () => useContext(SettingsContext);
