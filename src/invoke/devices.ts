import { invoke } from '@tauri-apps/api/core';

const isTauri = !!(window as any).__TAURI_INTERNALS__;

export const getDeviceStates = (): Promise<any[]> => {
  if (!isTauri) {
    return Promise.resolve([
      { id: "rtl-sdr", status: "connected", last_seen_ms: 123 },
      { id: "pluto-sdr", status: "connected", last_seen_ms: 123 },
      { id: "pico-2", status: "connected", last_seen_ms: 123 },
    ]);
  }
  return invoke('get_device_states');
};
export const popOutDevice = (deviceId: string): Promise<void> => {
  if (!isTauri) {
    console.log("Mock popOutDevice", deviceId);
    return Promise.resolve();
  }
  return invoke('pop_out_device', { deviceId });
};
