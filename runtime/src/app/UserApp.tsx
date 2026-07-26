import { ChangeEvent, DragEvent, useCallback, useEffect, useRef, useState } from 'react';
import { CanvasStage } from '../ui/components/CanvasStage';
import { useKeyboardInput } from '../ui/hooks/useKeyboardInput';
import { useRuntimeShell } from '../ui/hooks/useRuntimeShell';
import { uploadGamePackage, type UploadResponse } from './uploadGame';

type PagePhase = 'idle' | 'uploading' | 'parsing' | 'booting' | 'ready' | 'failed';

type StepState = 'pending' | 'active' | 'done';

const BUSY_PHASES: readonly PagePhase[] = ['uploading', 'parsing', 'booting'];

function isBusy(phase: PagePhase): boolean {
  return BUSY_PHASES.includes(phase);
}

function stepState(phase: PagePhase, step: 'upload' | 'parse' | 'boot'): StepState {
  const order: PagePhase[] = ['uploading', 'parsing', 'booting'];
  const stepPhase = step === 'upload' ? 'uploading' : step === 'parse' ? 'parsing' : 'booting';
  if (phase === stepPhase) {
    return 'active';
  }
  return order.indexOf(phase) > order.indexOf(stepPhase) || phase === 'ready' ? 'done' : 'pending';
}

function isSupportedGameFile(file: File): boolean {
  const name = file.name.toLowerCase();
  return name.endsWith('.exe') || name.endsWith('.zip');
}

function formatPlayTime(playMs: number): string {
  const totalSeconds = Math.floor(playMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const mm = String(minutes).padStart(2, '0');
  const ss = String(seconds).padStart(2, '0');
  return hours > 0 ? `${hours}:${mm}:${ss}` : `${mm}:${ss}`;
}

function jumpKeyLabel(virtualKey: number | undefined): string {
  switch (virtualKey) {
    case 0x10:
      return 'Shift';
    case 0x20:
      return 'Space';
    case 0x0d:
      return 'Enter';
    default:
      return virtualKey != null && virtualKey >= 0x30 && virtualKey <= 0x5a
        ? String.fromCharCode(virtualKey)
        : 'Shift';
  }
}

export function UserApp(): JSX.Element {
  const shell = useRuntimeShell({ allowStaticFallback: false, initialPackagePath: '' });
  const keyboard = useKeyboardInput();
  const keyboardRef = useRef(keyboard);
  const [phase, setPhase] = useState<PagePhase>('idle');
  const [fileName, setFileName] = useState<string | null>(null);
  const [uploadPercent, setUploadPercent] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [compatibility, setCompatibility] = useState<UploadResponse['compatibility'] | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const [playMs, setPlayMs] = useState(0);
  const playTimerRef = useRef({ accumulatedMs: 0, runStartMs: null as number | null });
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    keyboardRef.current = keyboard;
  }, [keyboard]);

  const ready = phase === 'ready' && shell.runtimeReady;
  const running = ready && shell.autoTickRunning;

  const readPlayMs = useCallback((): number => {
    const timer = playTimerRef.current;
    return timer.accumulatedMs + (timer.runStartMs != null ? Date.now() - timer.runStartMs : 0);
  }, []);

  // Accumulate play time only while the runtime is actually ticking.
  useEffect(() => {
    const timer = playTimerRef.current;
    if (running && timer.runStartMs == null) {
      timer.runStartMs = Date.now();
    }
    if (!running && timer.runStartMs != null) {
      timer.accumulatedMs += Date.now() - timer.runStartMs;
      timer.runStartMs = null;
    }
    setPlayMs(readPlayMs());
    if (!running) {
      return;
    }
    const handle = setInterval(() => setPlayMs(readPlayMs()), 500);
    return () => clearInterval(handle);
  }, [running, readPlayMs]);

  const resetPlayTimer = useCallback(() => {
    playTimerRef.current = {
      accumulatedMs: 0,
      runStartMs: playTimerRef.current.runStartMs != null ? Date.now() : null,
    };
    setPlayMs(0);
  }, []);

  const startUpload = useCallback(async (file: File) => {
    if (!isSupportedGameFile(file)) {
      setError('仅支持 .exe 或 .zip 文件。');
      setPhase('failed');
      return;
    }

    shell.stopAutoTick();
    playTimerRef.current = { accumulatedMs: 0, runStartMs: null };
    setPlayMs(0);
    setError(null);
    setCompatibility(null);
    setFileName(file.name);
    setUploadPercent(0);
    setPhase('uploading');

    try {
      const uploaded = await uploadGamePackage(file, (progress) => {
        setUploadPercent(progress.percent);
        setPhase(progress.phase === 'processing' ? 'parsing' : 'uploading');
      });
      setPhase('booting');
      await shell.loadCurrentPackage(keyboardRef, uploaded.package_url);
      setCompatibility(uploaded.compatibility);
      setPhase('ready');
    } catch (uploadError) {
      setError(uploadError instanceof Error ? uploadError.message : String(uploadError));
      setPhase('failed');
    }
  }, [shell]);

  function handleFileChange(event: ChangeEvent<HTMLInputElement>): void {
    const file = event.target.files?.[0] ?? null;
    event.target.value = '';
    if (file) {
      void startUpload(file);
    }
  }

  function handleDrop(event: DragEvent<HTMLElement>): void {
    event.preventDefault();
    setDragActive(false);
    const file = event.dataTransfer.files?.[0] ?? null;
    if (file && !isBusy(phase)) {
      void startUpload(file);
    }
  }

  function handleChangeGame(): void {
    shell.stopAutoTick();
    playTimerRef.current = { accumulatedMs: 0, runStartMs: null };
    setPlayMs(0);
    setFileName(null);
    setError(null);
    setCompatibility(null);
    setPhase('idle');
  }

  async function handleReset(): Promise<void> {
    await shell.resetRuntime();
    resetPlayTimer();
  }

  const deaths = shell.snapshot?.deaths ?? 0;
  const roomName = shell.snapshot?.roomName ?? null;
  const jumpKey = jumpKeyLabel(shell.snapshot?.inputTrace?.jumpButtonKey);
  const busy = isBusy(phase);

  const steps: Array<{ key: 'upload' | 'parse' | 'boot'; label: string; detail: string | null }> = [
    { key: 'upload', label: '上传游戏包', detail: phase === 'uploading' ? `${uploadPercent}%` : null },
    { key: 'parse', label: '解析与验证', detail: phase === 'parsing' ? '服务器处理中…' : null },
    { key: 'boot', label: '启动运行时', detail: phase === 'booting' ? '加载资源中…' : null },
  ];

  return (
    <main className="user-app">
      <header className="user-topbar">
        <p className="user-brand">
          <span className="user-brand-dot" aria-hidden="true" />
          IWanna GM8 Web Engine
          <em>Beta</em>
        </p>
        <a href="/shell">Shell 诊断</a>
      </header>

      {!ready
        ? (
          <section className="user-hero" aria-labelledby="user-title">
            <h1 id="user-title">在浏览器中运行 I Wanna 游戏</h1>
            <p className="user-intro">
              上传原始游戏的 EXE 文件，或包含完整游戏目录的 ZIP 文件。
              解析与验证完成后，游戏将直接在浏览器中启动。
            </p>

            {busy
              ? (
                <div className="user-progress" role="status" aria-live="polite">
                  <p className="user-progress-file">{fileName}</p>
                  <ol className="user-steps">
                    {steps.map((step) => {
                      const state = stepState(phase, step.key);
                      return (
                        <li key={step.key} className={`user-step is-${state}`}>
                          <span className="user-step-marker" aria-hidden="true" />
                          <span className="user-step-label">{step.label}</span>
                          <span className="user-step-detail">
                            {state === 'done' ? '完成' : step.detail}
                          </span>
                          {step.key === 'upload' && state === 'active'
                            ? (
                              <span
                                className="user-step-bar"
                                role="progressbar"
                                aria-valuemin={0}
                                aria-valuemax={100}
                                aria-valuenow={uploadPercent}
                              >
                                <span style={{ width: `${uploadPercent}%` }} />
                              </span>
                            )
                            : null}
                          {step.key !== 'upload' && state === 'active'
                            ? (
                              <span className="user-step-bar is-indeterminate" aria-hidden="true">
                                <span />
                              </span>
                            )
                            : null}
                        </li>
                      );
                    })}
                  </ol>
                </div>
              )
              : (
                <label
                  className={`user-dropzone ${dragActive ? 'is-drag' : ''}`}
                  onDragOver={(event) => {
                    event.preventDefault();
                    setDragActive(true);
                  }}
                  onDragLeave={() => setDragActive(false)}
                  onDrop={handleDrop}
                >
                  <input
                    ref={fileInputRef}
                    type="file"
                    accept=".exe,.zip,application/zip,application/x-msdownload"
                    aria-label="游戏包"
                    onChange={handleFileChange}
                  />
                  <span className="user-dropzone-icon" aria-hidden="true" />
                  <span className="user-dropzone-title">拖入游戏包，或点击选择文件</span>
                  <span className="user-dropzone-hint">.exe / .zip · 最大 512 MiB</span>
                </label>
              )}

            {phase === 'failed'
              ? (
                <div className="user-error" role="alert">
                  <p>{error ?? shell.error ?? '未能启动这个游戏。'}</p>
                </div>
              )
              : null}

            <p className="user-meta">仅解析上传内容，不会执行 EXE 或 DLL。</p>
          </section>
        )
        : null}

      <section className={`user-stage ${ready ? '' : 'is-hidden'}`} aria-label="游戏画面">
        <div className="user-hud">
          <p className="user-hud-room">
            <span className={`user-hud-dot ${running ? 'is-running' : ''}`} aria-hidden="true" />
            <span className="user-hud-room-name">{roomName ?? fileName ?? '运行中'}</span>
          </p>
          <dl className="user-hud-stats">
            <div className="user-hud-stat">
              <dt>死亡</dt>
              <dd>{deaths}</dd>
            </div>
            <div className="user-hud-stat">
              <dt>时间</dt>
              <dd>{formatPlayTime(playMs)}</dd>
            </div>
          </dl>
          <div className="user-hud-actions">
            <button type="button" onClick={() => shell.togglePause(keyboardRef)}>
              {shell.autoTickRunning ? '暂停' : '继续'}
            </button>
            <button type="button" onClick={() => void handleReset()}>重置</button>
            <button type="button" onClick={handleChangeGame}>换个游戏</button>
          </div>
        </div>

        <div className="user-stage-canvas">
          <CanvasStage
            ref={shell.canvasRef}
            error={null}
            width={shell.displayWidth}
            height={shell.displayHeight}
          />
        </div>

        {shell.error
          ? (
            <div className="user-error" role="alert">
              <p>{shell.error}</p>
            </div>
          )
          : null}

        <footer className="user-stage-footer">
          {compatibility === 'partial'
            ? <p className="user-compat">部分兼容：某些房间或 GM8 功能可能仍不可用。</p>
            : null}
          <p className="user-controls-hint">← → 移动 · {jumpKey} 跳跃 · R 重开</p>
        </footer>
      </section>
    </main>
  );
}
