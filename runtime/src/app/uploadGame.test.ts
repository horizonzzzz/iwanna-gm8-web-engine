import { afterEach, describe, expect, it, vi } from 'vitest';
import { uploadGamePackage, type UploadProgress } from './uploadGame';

type Listener = (event?: unknown) => void;

class FakeXhr {
  static last: FakeXhr | null = null;

  upload = {
    listeners: new Map<string, Listener>(),
    addEventListener(type: string, listener: Listener): void {
      this.listeners.set(type, listener);
    },
  };

  listeners = new Map<string, Listener>();
  opened: [string, string] | null = null;
  sentBody: FormData | null = null;
  status = 0;
  response: unknown = null;

  constructor() {
    FakeXhr.last = this;
  }

  open(method: string, url: string): void {
    this.opened = [method, url];
  }

  addEventListener(type: string, listener: Listener): void {
    this.listeners.set(type, listener);
  }

  send(body: FormData): void {
    this.sentBody = body;
  }

  emitUploadProgress(loaded: number, total: number): void {
    this.upload.listeners.get('progress')?.({ lengthComputable: true, loaded, total });
  }

  finish(status: number, response: unknown): void {
    this.status = status;
    this.response = response;
    this.listeners.get('load')?.();
  }
}

function installFakeXhr(): void {
  vi.stubGlobal('XMLHttpRequest', FakeXhr as unknown as typeof XMLHttpRequest);
}

afterEach(() => {
  vi.unstubAllGlobals();
  FakeXhr.last = null;
});

describe('uploadGamePackage', () => {
  it('posts the file as form data and resolves with the parsed response', async () => {
    installFakeXhr();
    const progress: UploadProgress[] = [];
    const pending = uploadGamePackage(
      new File(['data'], 'game.exe'),
      (update) => progress.push(update)
    );
    const xhr = FakeXhr.last!;

    xhr.emitUploadProgress(50, 200);
    xhr.emitUploadProgress(200, 200);
    xhr.finish(200, {
      id: 'g1',
      status: 'ready',
      compatibility: 'supported',
      package_url: '/games/g1',
      warnings: [],
    });

    await expect(pending).resolves.toMatchObject({ package_url: '/games/g1' });
    expect(xhr.opened).toEqual(['POST', '/api/v1/games']);
    expect(xhr.sentBody?.get('game')).toBeInstanceOf(File);
    expect(progress).toEqual([
      { phase: 'uploading', percent: 25 },
      { phase: 'processing', percent: 100 },
    ]);
  });

  it('rejects with the server-provided error message', async () => {
    installFakeXhr();
    const pending = uploadGamePackage(new File(['data'], 'game.exe'), () => {});

    FakeXhr.last!.finish(413, JSON.stringify({ error: '包体超过 512 MiB 限制。' }));

    await expect(pending).rejects.toThrow('包体超过 512 MiB 限制。');
  });

  it('falls back to an HTTP status message when the body is not JSON', async () => {
    installFakeXhr();
    const pending = uploadGamePackage(new File(['data'], 'game.exe'), () => {});

    FakeXhr.last!.finish(502, '<html>bad gateway</html>');

    await expect(pending).rejects.toThrow('上传失败（HTTP 502）');
  });

  it('rejects when the connection drops mid-upload', async () => {
    installFakeXhr();
    const pending = uploadGamePackage(new File(['data'], 'game.exe'), () => {});

    FakeXhr.last!.listeners.get('error')?.();

    await expect(pending).rejects.toThrow('网络错误，上传中断。');
  });
});
