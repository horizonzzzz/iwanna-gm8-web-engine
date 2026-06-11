import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { App } from './app/App';

describe('runtime app bootstrap', () => {
  it('renders the runtime shell title and load controls', () => {
    render(<App />);

    expect(screen.getByText('IWanna Runtime Shell')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Package' })).toHaveValue('/packages/sample');
    expect(screen.getByRole('button', { name: 'Load Package' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Room' })).toBeDisabled();
    expect(screen.getByText(/Execution path:/)).toBeInTheDocument();
  });
});
