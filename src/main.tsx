import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './assets/tokens.css';
import './assets/index.css';

// Import MD3 web components available in v2.4.1
import '@material/web/slider/slider.js';
import '@material/web/switch/switch.js';
import '@material/web/chips/filter-chip.js';
import '@material/web/select/outlined-select.js';
import '@material/web/select/select-option.js';
import '@material/web/textfield/outlined-text-field.js';
import '@material/web/button/filled-button.js';
import '@material/web/button/outlined-button.js';
import '@material/web/button/filled-tonal-button.js';
import '@material/web/divider/divider.js';
import '@material/web/progress/linear-progress.js';
import '@material/web/labs/card/elevated-card.js';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
