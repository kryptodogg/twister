import { invoke } from '@tauri-apps/api/core';

const isTauri = !!(window as any).__TAURI_INTERNALS__;

export const getGpioAssignments = (): Promise<any[]> => {
  if (!isTauri) {
    return Promise.resolve([
      { pin: 4, function: "PPS from Pico 2", direction: "IN", pull: "DOWN", connected_to: "Pico 2 GPIO 0", active_state: "HIGH" },
    ]);
  }
  return invoke('get_gpio_assignments');
};
export const saveGpioAssignments = (pins: any[]): Promise<void> => {
  if (!isTauri) {
    console.log("Mock saveGpioAssignments", pins);
    return Promise.resolve();
  }
  return invoke('save_gpio_assignments', { pins });
};
export const exportGpioConfig = (path: string): Promise<void> => {
  if (!isTauri) {
    console.log("Mock exportGpioConfig", path);
    return Promise.resolve();
  }
  return invoke('export_gpio_config', { path });
};
