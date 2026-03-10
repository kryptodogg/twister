import React from 'react';
import ReactDOM from 'react-dom/client';
import WaveshaperMetrics from '../waveshaper_metrics';
import '../index.css';

ReactDOM.createRoot(document.getElementById('root')).render(
  <React.StrictMode>
    <WaveshaperMetrics wsUrl="ws://localhost:8080/metrics" />
  </React.StrictMode>
);
