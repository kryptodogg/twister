import React from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

export const ResizeOverlay: React.FC = () => {
  const onResize = (direction: string) => {
    // In Tauri v2, the startResize API is slightly different
    // but the principle remains. 
    // Usually it's `window.startResize(direction)`
    (appWindow as any).startResize(direction);
  };

  return (
    <>
      <div 
        className="resize-bottom" 
        onMouseDown={() => onResize('Bottom')}
        style={{ 
          position: 'fixed', bottom: 0, left: 0, right: 0, height: 4, 
          cursor: 'ns-resize', zIndex: 9999, background: 'transparent' 
        }} 
      />
      <div 
        className="resize-right" 
        onMouseDown={() => onResize('Right')}
        style={{ 
          position: 'fixed', right: 0, top: 0, bottom: 0, width: 4, 
          cursor: 'ew-resize', zIndex: 9999, background: 'transparent' 
        }} 
      />
      <div 
        className="resize-bottom-right" 
        onMouseDown={() => onResize('BottomRight')}
        style={{ 
          position: 'fixed', right: 0, bottom: 0, width: 8, height: 8, 
          cursor: 'nwse-resize', zIndex: 10000, background: 'transparent' 
        }} 
      />
    </>
  );
};
