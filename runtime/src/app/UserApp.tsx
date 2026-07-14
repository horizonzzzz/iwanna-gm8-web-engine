import { FormEvent, useEffect, useRef, useState } from 'react';
import { CanvasStage } from '../ui/components/CanvasStage';
import { useKeyboardInput } from '../ui/hooks/useKeyboardInput';
import { useRuntimeShell } from '../ui/hooks/useRuntimeShell';

type UploadResponse = {
  id: string;
  status: 'ready';
  compatibility: 'supported' | 'partial' | 'blocked';
  package_url: string;
  warnings: string[];
};

type UploadError = {
  error?: string;
};

type PageStatus = 'idle' | 'uploading' | 'booting' | 'ready' | 'failed';

async function apiErrorMessage(response: Response): Promise<string> {
  const body = await response.json().catch(() => null) as UploadError | null;
  return body?.error ?? `上传失败（HTTP ${response.status}）`;
}

function statusMessage(status: PageStatus, error: string | null): string {
  switch (status) {
    case 'uploading':
      return '正在上传游戏包…';
    case 'booting':
      return '解析完成，正在启动游戏…';
    case 'ready':
      return '游戏已启动。';
    case 'failed':
      return error ?? '未能启动这个游戏。';
    case 'idle':
      return '选择一个 .exe 或 .zip 游戏包。';
  }
}

export function UserApp(): JSX.Element {
  const shell = useRuntimeShell({ allowStaticFallback: false, initialPackagePath: '' });
  const keyboard = useKeyboardInput();
  const keyboardRef = useRef(keyboard);
  const [file, setFile] = useState<File | null>(null);
  const [status, setStatus] = useState<PageStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [compatibility, setCompatibility] = useState<UploadResponse['compatibility'] | null>(null);

  useEffect(() => {
    keyboardRef.current = keyboard;
  }, [keyboard]);

  async function handleUpload(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    if (!file) {
      setError('请先选择一个 .exe 或 .zip 文件。');
      setStatus('failed');
      return;
    }

    shell.stopAutoTick();
    setError(null);
    setCompatibility(null);
    setStatus('uploading');
    const form = new FormData();
    form.append('game', file);

    try {
      const response = await fetch('/api/v1/games', { method: 'POST', body: form });
      if (!response.ok) {
        throw new Error(await apiErrorMessage(response));
      }
      const uploaded = await response.json() as UploadResponse;
      setStatus('booting');
      await shell.loadCurrentPackage(keyboardRef, uploaded.package_url);
      setCompatibility(uploaded.compatibility);
      setStatus('ready');
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : String(uploadError));
      setStatus('failed');
    }
  }

  const ready = status === 'ready' && shell.runtimeReady;
  const busy = status === 'uploading' || status === 'booting';

  return (
    <main className="beta-app">
      <section className="beta-upload" aria-labelledby="beta-title">
        <div>
          <p className="beta-kicker">IWANNA GM8 WEB ENGINE / BETA</p>
          <h1 id="beta-title">在浏览器中运行 IWanna 游戏</h1>
          <p className="beta-intro">
            上传原始 IWanna 游戏的 EXE 文件，或包含完整游戏目录的 ZIP 文件。系统完成解析与验证后，将在浏览器 Canvas 中启动游戏。
          </p>
        </div>

        <form className="beta-form" onSubmit={(event) => void handleUpload(event)}>
          <label htmlFor="game-upload">游戏包</label>
          <input
            id="game-upload"
            type="file"
            accept=".exe,.zip,application/zip,application/x-msdownload"
            disabled={busy}
            onChange={(event) => setFile(event.target.files?.[0] ?? null)}
          />
          <button type="submit" disabled={busy || !file}>
            {busy ? '处理中…' : ready ? '载入另一个游戏' : '开始游戏'}
          </button>
        </form>

        <div
          className={`beta-status beta-status-${status}`}
          role={status === 'failed' ? 'alert' : 'status'}
          aria-live="polite"
        >
          <span aria-hidden="true" />
          <p>{statusMessage(status, error ?? shell.error)}</p>
        </div>

        <div className="beta-meta">
          <p>最大 512 MiB · 仅解析上传内容，不执行 EXE 或 DLL</p>
          {compatibility === 'partial'
            ? <p>部分兼容：某些房间或 GM8 功能可能仍不可用。</p>
            : null}
          <a href="/shell">需要诊断？打开 Shell</a>
        </div>
      </section>

      <section className={`beta-player ${ready ? 'is-ready' : ''}`} aria-label="游戏画面">
        <header>
          <span className="beta-player-mark" aria-hidden="true" />
          <p>{ready ? 'RUNNING' : 'WAITING FOR GAME'}</p>
          <button
            type="button"
            disabled={!ready}
            onClick={() => void shell.resetRuntime()}
          >
            重置
          </button>
        </header>
        <div className="beta-player-canvas">
          <CanvasStage
            ref={shell.canvasRef}
            error={null}
            width={shell.displayWidth}
            height={shell.displayHeight}
          />
          {!ready
            ? (
              <div className="beta-empty">
                <span className="beta-empty-signal" aria-hidden="true" />
                <p className="beta-empty-label">等待游戏包</p>
                <p>选择一个游戏包，Canvas 会在这里启动。</p>
              </div>
            )
            : null}
        </div>
      </section>
    </main>
  );
}
