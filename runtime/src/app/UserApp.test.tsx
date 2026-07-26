import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { UserApp } from './UserApp';
import type { UploadProgress, UploadResponse } from './uploadGame';

const mocks = vi.hoisted(() => ({
  loadCurrentPackage: vi.fn(),
  stopAutoTick: vi.fn(),
  resetRuntime: vi.fn(),
  togglePause: vi.fn(),
  uploadGamePackage: vi.fn(),
}));

vi.mock('./uploadGame', () => ({
  uploadGamePackage: mocks.uploadGamePackage,
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
    togglePause: mocks.togglePause,
    runtimeReady: true,
    autoTickRunning: true,
    error: null,
    canvasRef: { current: null },
    displayWidth: 800,
    displayHeight: 600,
    snapshot: {
      tick: 90,
      roomId: 3,
      roomName: 'rStage01',
      deaths: 7,
      diagnostics: [],
      inputTrace: {
        jumpButtonKey: 0x10,
        jumpPressed: false,
        jumpJustPressed: false,
        jumpJustReleased: false,
        activeKeys: [],
      },
      player: null,
    },
  }),
}));

const uploadResponse: UploadResponse = {
  id: 'fixture',
  status: 'ready',
  compatibility: 'partial',
  package_url: '/games/fixture',
  warnings: [],
};

function selectFile(name = 'fixture.exe'): void {
  const file = new File(['fixture'], name, { type: 'application/octet-stream' });
  fireEvent.change(screen.getByLabelText('游戏包'), { target: { files: [file] } });
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe('UserApp', () => {
  it('uploads the selected file and boots the returned package', async () => {
    mocks.uploadGamePackage.mockResolvedValue(uploadResponse);
    mocks.loadCurrentPackage.mockResolvedValue({});
    render(<UserApp />);

    selectFile();

    await waitFor(() => {
      expect(mocks.loadCurrentPackage).toHaveBeenCalledWith(
        expect.objectContaining({ current: expect.any(Object) }),
        '/games/fixture'
      );
    });
    expect(mocks.stopAutoTick).toHaveBeenCalled();
    expect(screen.getByText('死亡')).toBeInTheDocument();
    expect(screen.getByText('7')).toBeInTheDocument();
    expect(screen.getByText('rStage01')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重置' })).toBeInTheDocument();
    expect(screen.getByText('部分兼容：某些房间或 GM8 功能可能仍不可用。')).toBeInTheDocument();
  });

  it('shows upload percent and the parsing step while the request is in flight', async () => {
    let reportProgress: ((progress: UploadProgress) => void) | null = null;
    let finishUpload: ((response: UploadResponse) => void) | null = null;
    mocks.uploadGamePackage.mockImplementation(
      (_file: File, onProgress: (progress: UploadProgress) => void) => {
        reportProgress = onProgress;
        return new Promise<UploadResponse>((resolve) => {
          finishUpload = resolve;
        });
      }
    );
    mocks.loadCurrentPackage.mockResolvedValue({});
    render(<UserApp />);

    selectFile();
    expect(await screen.findByText('fixture.exe')).toBeInTheDocument();

    act(() => reportProgress?.({ phase: 'uploading', percent: 42 }));
    expect(screen.getByText('42%')).toBeInTheDocument();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '42');

    act(() => reportProgress?.({ phase: 'processing', percent: 100 }));
    expect(screen.getByText('服务器处理中…')).toBeInTheDocument();

    act(() => finishUpload?.(uploadResponse));
    await waitFor(() => {
      expect(mocks.loadCurrentPackage).toHaveBeenCalled();
    });
  });

  it('rejects unsupported file types without uploading', () => {
    render(<UserApp />);

    selectFile('savegame.dat');

    expect(mocks.uploadGamePackage).not.toHaveBeenCalled();
    expect(screen.getByRole('alert')).toHaveTextContent('仅支持 .exe 或 .zip 文件。');
  });

  it('surfaces upload failures returned by the server', async () => {
    mocks.uploadGamePackage.mockRejectedValue(new Error('包体超过 512 MiB 限制。'));
    render(<UserApp />);

    selectFile();

    expect(await screen.findByRole('alert')).toHaveTextContent('包体超过 512 MiB 限制。');
    expect(mocks.loadCurrentPackage).not.toHaveBeenCalled();
  });
});
