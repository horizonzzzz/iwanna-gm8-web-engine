import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { App } from './App';

describe('App', () => {
  it('renders canvas, hud, debug report, and inspector tabs together', () => {
    render(<App />);

    expect(screen.getByText('IWanna Runtime Shell')).toBeInTheDocument();
    expect(screen.getByText('Debug Report')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Package' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Rooms' })).toBeInTheDocument();
    expect(document.querySelector('#room-canvas')).not.toBeNull();
    expect(document.querySelector('#runtime-status')).not.toBeNull();
  });
});
