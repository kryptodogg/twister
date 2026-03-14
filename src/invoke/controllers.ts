import { invoke } from '@tauri-apps/api/core';

const isTauri = !!(window as any).__TAURI_INTERNALS__;

export const listControllers = (): Promise<any[]> => {
  if (!isTauri) {
    return Promise.resolve([
      { id: "joycon-l", name: "Joy-Con (L)", connection_type: "Bluetooth" },
      { id: "joycon-r", name: "Joy-Con (R)", connection_type: "Bluetooth" },
      { id: "dualsense", name: "DualSense", connection_type: "USB" },
    ]);
  }
  return invoke('list_controllers');
};
export const getControllerState = (id: string): Promise<any> => {
  if (!isTauri) {
    return Promise.resolve({
      battery_level: 0.75,
      accelerometer: [0.1, 9.8, 0.2],
      gyroscope: [0.0, 0.0, 0.0],
      buttons: [],
      sticks: [[0, 0]],
    });
  }
  return invoke('get_controller_state', { id });
};
export const testRumble = (id: string, side: string, intensity: number): Promise<void> => {
  if (!isTauri) return Promise.resolve();
  return invoke('test_rumble', { id, side, intensity });
};
export const testHaptic = (id: string, side: string, intensity: number): Promise<void> => {
  if (!isTauri) return Promise.resolve();
  return invoke('test_haptic', { id, side, intensity });
};
export const setLightbarColor = (id: string, r: number, g: number, b: number): Promise<void> => {
  if (!isTauri) return Promise.resolve();
  return invoke('set_lightbar_color', { id, r, g, b });
};
