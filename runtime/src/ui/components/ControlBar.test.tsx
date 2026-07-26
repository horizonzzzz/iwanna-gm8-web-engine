import type { ComponentProps } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { ControlBar } from './ControlBar';

type Handlers = {
  onLoad: ReturnType<typeof vi.fn>;
  onPauseToggle: ReturnType<typeof vi.fn>;
  onReset: ReturnType<typeof vi.fn>;
  onRoomChange: ReturnType<typeof vi.fn>;
  onPackagePathChange: ReturnType<typeof vi.fn>;
  onPackageSelect: ReturnType<typeof vi.fn>;
};

function renderControlBar(overrides: Partial<ComponentProps<typeof ControlBar>> = {}): Handlers {
  const handlers: Handlers = {
    onLoad: vi.fn(),
    onPauseToggle: vi.fn(),
    onReset: vi.fn(),
    onRoomChange: vi.fn(),
    onPackagePathChange: vi.fn(),
    onPackageSelect: vi.fn(),
  };

  render(
    <ControlBar
      packagePath="/packages/sample"
      onPackagePathChange={handlers.onPackagePathChange}
      packageOptions={[]}
      onPackageSelect={handlers.onPackageSelect}
      onLoad={handlers.onLoad}
      roomOptions={[{ id: 143, name: 'sampleroom01' }]}
      selectedRoomId={143}
      onRoomChange={handlers.onRoomChange}
      autoTickRunning={true}
      runtimeReady={true}
      onPauseToggle={handlers.onPauseToggle}
      onReset={handlers.onReset}
      backendStatus="WASM bridge available"
      {...overrides}
    />
  );

  return handlers;
}

afterEach(() => {
  cleanup();
});

describe('ControlBar', () => {
  it('lets the user change package path, load, pause, and reset', () => {
    const handlers = renderControlBar();

    fireEvent.change(screen.getByRole('textbox', { name: 'Package path' }), {
      target: { value: '/packages/next' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Load Package' }));
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    fireEvent.change(screen.getByRole('combobox', { name: 'Room' }), {
      target: { value: '143' },
    });

    expect(screen.queryByRole('combobox', { name: 'Difficulty' })).not.toBeInTheDocument();
    expect(handlers.onPackagePathChange).toHaveBeenCalledWith('/packages/next');
    expect(handlers.onLoad).toHaveBeenCalled();
    expect(handlers.onPauseToggle).toHaveBeenCalled();
    expect(handlers.onReset).toHaveBeenCalled();
    expect(handlers.onRoomChange).toHaveBeenCalledWith(143);
  });

  it('disables the package picker when no packages were discovered', () => {
    renderControlBar();

    const picker = screen.getByRole('combobox', { name: 'Package' });
    expect(picker).toBeDisabled();
    expect(picker).toHaveTextContent('No packages under public/packages');
  });

  it('selects a discovered package and ignores the placeholder option', () => {
    const handlers = renderControlBar({
      packageOptions: [
        { path: '/packages/gm8-core/IWBT_Dife', label: 'gm8-core/IWBT_Dife', title: 'I wanna be the Dife' },
        { path: '/packages/gm8-core/Crimson', label: 'gm8-core/Crimson', title: null },
      ],
    });

    const picker = screen.getByRole('combobox', { name: 'Package' });
    expect(screen.getByRole('option', { name: 'gm8-core/IWBT_Dife' })).toBeInTheDocument();
    // The current path is not in the catalog, so the placeholder keeps it visible.
    expect(screen.getByRole('option', { name: 'Custom: /packages/sample' })).toBeInTheDocument();

    fireEvent.change(picker, { target: { value: '/packages/gm8-core/IWBT_Dife' } });
    expect(handlers.onPackageSelect).toHaveBeenCalledWith('/packages/gm8-core/IWBT_Dife');

    fireEvent.change(picker, { target: { value: '' } });
    expect(handlers.onPackageSelect).toHaveBeenCalledTimes(1);
  });
});
