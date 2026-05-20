import './styles.css';
import { createRuntimeShell } from './ui/shell';

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('Missing app root');
}

createRuntimeShell(app);
