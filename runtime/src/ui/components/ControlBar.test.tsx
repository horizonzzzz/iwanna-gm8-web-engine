import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { ControlBar } from './ControlBar';

describe('ControlBar', () => {
  it('lets the user change package path, load, pause, and reset', () => {
    const onLoad = vi.fn();
    const onPauseToggle = vi.fn();
    const onReset = vi.fn();
    const onRoomChange = vi.fn();
    const onPackagePathChange = vi.fn();

    render(
      <ControlBar
        packagePath="/packages/sample"
        onPackagePathChange={onPackagePathChange}
        onLoad={onLoad}
        roomOptions={[{ id: 143, name: 'sampleroom01' }]}
        selectedRoomId={143}
        onRoomChange={onRoomChange}
        autoTickRunning={true}
        runtimeReady={true}
        onPauseToggle={onPauseToggle}
        onReset={onReset}
        backendStatus="WASM bridge available"
      />
    );

    fireEvent.change(screen.getByRole('textbox', { name: 'Package' }), {
      target: { value: '/packages/next' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Load Package' }));
    fireEvent.click(screen.getByRole('button', { name: 'Pause' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reset' }));
    fireEvent.change(screen.getByRole('combobox', { name: 'Room' }), {
      target: { value: '143' },
    });

    expect(onPackagePathChange).toHaveBeenCalledWith('/packages/next');
    expect(onLoad).toHaveBeenCalled();
    expect(onPauseToggle).toHaveBeenCalled();
    expect(onReset).toHaveBeenCalled();
    expect(onRoomChange).toHaveBeenCalledWith(143);
  });
});
