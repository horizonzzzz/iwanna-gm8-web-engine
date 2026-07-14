import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { UserApp } from './UserApp';

const mocks = vi.hoisted(() => ({
  loadCurrentPackage: vi.fn(),
  stopAutoTick: vi.fn(),
  resetRuntime: vi.fn(),
}));

vi.mock('../ui/hooks/useKeyboardInput', () => ({
  useKeyboardInput: () => ({
    left: false,
    right: false,
    jump: false,
    restart: false,
    keysHeld: [],
    keysPressed: [],
    keysReleased: [],
    clearEdgeKeys: vi.fn(),
  }),
}));

vi.mock('../ui/hooks/useRuntimeShell', () => ({
  useRuntimeShell: () => ({
    stopAutoTick: mocks.stopAutoTick,
    loadCurrentPackage: mocks.loadCurrentPackage,
    resetRuntime: mocks.resetRuntime,
    runtimeReady: true,
    error: null,
    canvasRef: { current: null },
    displayWidth: 800,
    displayHeight: 600,
  }),
}));

afterEach(() => {
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('UserApp', () => {
  it('loads the package URL returned by the upload API', async () => {
    mocks.loadCurrentPackage.mockResolvedValue({});
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        id: 'fixture',
        status: 'ready',
        compatibility: 'partial',
        package_url: '/games/fixture',
        warnings: [],
      }),
    }));
    render(<UserApp />);
    const file = new File(['fixture'], 'fixture.exe', { type: 'application/octet-stream' });

    fireEvent.change(screen.getByLabelText('游戏包'), { target: { files: [file] } });
    fireEvent.click(screen.getByRole('button', { name: '开始游戏' }));

    await waitFor(() => {
      expect(mocks.loadCurrentPackage).toHaveBeenCalledWith(
        expect.objectContaining({ current: expect.any(Object) }),
        '/games/fixture'
      );
    });
    expect(screen.getByText('游戏已启动。')).toBeInTheDocument();
  });
});
