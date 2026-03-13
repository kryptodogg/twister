import React from 'react';
import { Header } from './Header';
import { ResizeOverlay } from './ResizeOverlay';

function App() {
  return (
    <div style={{ backgroundColor: 'transparent', minHeight: '100vh' }}>
      <Header />
      <div style={{ padding: '60px 20px', color: 'white', textAlign: 'center' }}>
        <h1>Synesthesia</h1>
        <p>Full-Spectrum Forensic Platform</p>
      </div>
      <ResizeOverlay />
    </div>
  );
}

export default App;
