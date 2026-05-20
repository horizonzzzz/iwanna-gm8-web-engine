import './styles.css';
import { createRuntimeShell } from './ui/shell';
import { loadDefaultWasmRuntimeBridge } from './runtime/wasmBridge';

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('Missing app root');
}

createRuntimeShell(app, {
  loadWasmBridge: loadDefaultWasmRuntimeBridge
});
