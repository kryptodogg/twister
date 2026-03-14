import { useState } from 'react';
import { DragRegion } from './components/ui/DragRegion';
import { NavRail } from './components/ui/NavRail';
import { Hardware } from './pages/Hardware';
import { GPIO } from './pages/GPIO';
import { Controllers } from './pages/Controllers';
import { SettingsProvider } from './context/SettingsContext';
import { DeviceProvider } from './context/DeviceContext';

const PAGES = [
  { id: 'hardware',    label: 'Hardware',    icon: 'sensors' },
  { id: 'gpio',        label: 'GPIO',        icon: 'developer_board' },
  { id: 'controllers', label: 'Controllers', icon: 'sports_esports' },
];

export default function App() {
  const [active, setActive] = useState('hardware');

  const renderPage = () => {
    switch (active) {
      case 'hardware': return <Hardware />;
      case 'gpio': return <GPIO />;
      case 'controllers': return <Controllers />;
      default: return <Hardware />;
    }
  };

  return (
    <SettingsProvider>
      <DeviceProvider>
        <div className="h-[100dvh] overflow-hidden bg-transparent grid grid-cols-[72px_1fr]">
          <DragRegion />
          <NavRail pages={PAGES} active={active} onNavigate={setActive} />
          <main className="overflow-y-auto p-6 pt-14 min-w-0">
            {renderPage()}
          </main>
        </div>
      </DeviceProvider>
    </SettingsProvider>
  );
}
