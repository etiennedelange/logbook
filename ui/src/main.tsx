import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './theme.css';
import App from './App';
import { initTheme, useApp } from './state/store';

useApp.setState({ theme: initTheme() });

// `pnpm dev:mock`: run in a plain browser against fake data (see dev/mock.ts). Dead code in production.
if (import.meta.env.MODE === 'mock') {
  const { installMock } = await import('./dev/mock');
  installMock();
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
