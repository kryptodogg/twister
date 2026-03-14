import { useState } from 'react';

export function DragRegion() {
  const [maximized, setMaximized] = useState(false);

  const handleMinimize = () => {
    try {
      const { getCurrentWindow } = require('@tauri-apps/api/window');
      getCurrentWindow().minimize();
    } catch (e) {}
  };
  const handleMaximize = async () => {
    try {
      const { getCurrentWindow } = require('@tauri-apps/api/window');
      const appWindow = getCurrentWindow();
      await appWindow.toggleMaximize();
      const isMax = await appWindow.isMaximized();
      setMaximized(isMax);
    } catch (e) {}
  };
  const handleClose = () => {
    try {
      const { getCurrentWindow } = require('@tauri-apps/api/window');
      getCurrentWindow().close();
    } catch (e) {}
  };

  return (
    <header className="fixed top-0 left-0 w-full h-10 flex items-center justify-between px-4 z-[100] drag-region bg-surface-header/50 backdrop-blur-md border-b border-white/5">
      <div className="flex items-center gap-2 no-drag">
        <span className="text-[10px] font-bold tracking-widest uppercase text-br-teal">Brick Road</span>
        <span className="text-[10px] px-1.5 py-0.5 rounded bg-br-teal/10 text-br-teal font-semibold">SETTINGS</span>
      </div>

      <div className="flex items-center gap-2 no-drag">
        <button
          onClick={handleMinimize}
          className="w-6 h-6 flex items-center justify-center rounded-full hover:bg-white/10 text-br-green transition-colors"
          title="Minimize"
        >
          <span className="material-symbols-outlined text-sm">minimize</span>
        </button>
        <button
          onClick={handleMaximize}
          className="w-6 h-6 flex items-center justify-center rounded-full hover:bg-white/10 text-br-tan transition-colors"
          title={maximized ? "Restore" : "Maximize"}
        >
          <span className="material-symbols-outlined text-sm">{maximized ? 'square' : 'maximize'}</span>
        </button>
        <button
          onClick={handleClose}
          className="w-6 h-6 flex items-center justify-center rounded-full hover:bg-red-500/20 text-red-400 transition-colors"
          title="Close"
        >
          <span className="material-symbols-outlined text-sm">close</span>
        </button>
      </div>
    </header>
  );
}
