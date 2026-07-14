import { App } from './App';
import { UserApp } from './UserApp';

export function RootApp(): JSX.Element {
  return window.location.pathname === '/shell' || window.location.pathname.startsWith('/shell/')
    ? <App />
    : <UserApp />;
}
