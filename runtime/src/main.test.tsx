import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { RootApp } from './app/RootApp';

afterEach(() => {
  cleanup();
  window.history.replaceState({}, '', '/');
});

describe('runtime app bootstrap', () => {
  it('renders the public upload page at root', () => {
    window.history.replaceState({}, '', '/');
    render(<RootApp />);

    expect(screen.getByRole('heading', { name: '在浏览器中运行 IWanna 游戏' })).toBeInTheDocument();
    expect(screen.getByLabelText('游戏包')).toHaveAttribute('accept', expect.stringContaining('.exe'));
    expect(screen.getByRole('button', { name: '开始游戏' })).toBeDisabled();
  });

  it('keeps the diagnostic shell at /shell', () => {
    window.history.replaceState({}, '', '/shell');
    render(<RootApp />);

    expect(screen.getByText('IWanna Runtime Shell')).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Package' })).toHaveValue('/packages/sample');
    expect(screen.getByRole('button', { name: 'Load Package' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Pause' })).toBeDisabled();
    expect(screen.getByRole('combobox', { name: 'Room' })).toBeDisabled();
    expect(screen.getByText(/Execution path:/)).toBeInTheDocument();
  });
});
