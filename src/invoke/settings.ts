import { invoke } from '@tauri-apps/api/core';

const isTauri = !!(window as any).__TAURI_INTERNALS__;

export const getSettings = (): Promise<any> => {
  if (!isTauri) {
    return Promise.resolve({
      devices: {
        "rtl-sdr": { "sample_rate": "2.4M", "ppm": 0, "gain_mode": "auto", "youloop": false },
        "pluto-sdr": { "mode": "rx", "tx_power": 0, "rx_gain": 40, "sample_rate": "2M" },
        "pico-2": { "pps_pin": 4, "serial_port": "AUTO", "uwb_mode": "disabled" }
      },
      gpio: []
    });
  }
  return invoke('get_settings');
};
export const saveSettings = (settings: any): Promise<void> => {
  if (!isTauri) {
    console.log("Mock saveSettings", settings);
    return Promise.resolve();
  }
  return invoke('save_settings', { settings });
};
