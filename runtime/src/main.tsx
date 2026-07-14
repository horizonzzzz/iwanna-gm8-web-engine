import React from 'react';
import ReactDOM from 'react-dom/client';
import './styles.css';
import { RootApp } from './app/RootApp';

const root = document.querySelector<HTMLDivElement>('#app');

if (!root) {
  throw new Error('Missing app root');
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <RootApp />
  </React.StrictMode>
);
