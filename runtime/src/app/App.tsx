import { useEffect, useMemo, useRef } from 'react';
import { CanvasStage } from '../ui/components/CanvasStage';
import { ControlBar } from '../ui/components/ControlBar';
import { DebugReportPanel } from '../ui/components/DebugReportPanel';
import { InspectorTabs } from '../ui/components/InspectorTabs';
import { RuntimeHud } from '../ui/components/RuntimeHud';
import { useDebugReport } from '../ui/hooks/useDebugReport';
import { useKeyboardInput } from '../ui/hooks/useKeyboardInput';
import { useRuntimeShell } from '../ui/hooks/useRuntimeShell';

export function App(): JSX.Element {
  const shell = useRuntimeShell();
  const keyboard = useKeyboardInput();
  const keyboardRef = useRef(keyboard);

  useEffect(() => {
    keyboardRef.current = keyboard;
  }, [keyboard]);

  const statusLabel = shell.error
    ? shell.error
    : shell.mode === 'wasm'
      ? `WASM runtime active: ${shell.snapshot?.roomName ?? 'room'} @ tick ${shell.snapshot?.tick ?? 0}`
      : shell.selectedRoomId != null
        ? `Static room viewer: ${shell.snapshot?.roomName ?? 'room'}`
        : 'Idle';

  const roomLabel = shell.snapshot?.roomId != null
    ? `${shell.snapshot.roomId}: ${shell.snapshot.roomName ?? 'room'}`
    : 'none';

  const hudCards = useMemo(
    () => [
      {
        id: 'runtime-status',
        label: 'Status',
        value: statusLabel,
      },
      {
        id: 'runtime-room',
        label: 'Room',
        value: `Room: ${roomLabel}`,
      },
      {
        id: 'runtime-tick',
        label: 'Tick',
        value: shell.snapshot ? `Tick: ${shell.snapshot.tick}` : 'Tick: 0',
      },
      {
        id: 'runtime-player',
        label: 'Player',
        value: shell.snapshot?.player ? `Player: x=${shell.snapshot.player.x}` : 'Player: unavailable',
      },
      {
        id: 'runtime-input',
        label: 'Input',
        value: shell.snapshot
          ? `Input: jumpKey=0x${shell.snapshot.inputTrace.jumpButtonKey.toString(16)} keys=[${shell.snapshot.inputTrace.activeKeys.join(',')}]`
          : 'Input: unavailable',
      },
      {
        id: 'runtime-events',
        label: 'Events',
        value: shell.snapshot?.diagnostics.filter((item) => item.includes('runtime-')).slice(-1)[0] ?? 'Events: none',
      },
      {
        id: 'runtime-diagnostics',
        label: 'Diagnostics',
        value: shell.snapshot ? `Diagnostics: ${Math.min(shell.snapshot.diagnostics.length, 8)} recent` : 'Diagnostics: none',
      },
      {
        id: 'runtime-performance',
        label: 'Frame',
        value: shell.performance
          ? `Frame: ${shell.performance.totalMs.toFixed(1)}ms`
          : 'Frame: unavailable',
      },
    ],
    [roomLabel, shell.performance, shell.snapshot, statusLabel]
  );

  const debugReport = useDebugReport({
    status: statusLabel,
    roomLabel,
    snapshot: shell.snapshot,
    performance: shell.performance,
  });

  return (
    <main className="min-h-screen bg-slate-950 text-slate-100">
      <ControlBar
        packagePath={shell.packagePath}
        onPackagePathChange={shell.setPackagePath}
        onLoad={() => void shell.loadCurrentPackage(keyboardRef)}
        roomOptions={shell.roomOptions}
        selectedRoomId={shell.selectedRoomId}
        selectedDifficulty={shell.selectedDifficulty}
        onRoomChange={shell.setSelectedRoomId}
        onDifficultyChange={shell.setSelectedDifficulty}
        autoTickRunning={shell.autoTickRunning}
        runtimeReady={shell.runtimeReady}
        onPauseToggle={() => shell.togglePause(keyboardRef)}
        onReset={() => void shell.resetRuntime()}
        backendStatus={shell.backendStatus}
      />
      <section className="mx-auto flex w-full max-w-[1800px] flex-col gap-4 px-6 py-6">
        <CanvasStage ref={shell.canvasRef} error={shell.error} width={shell.displayWidth} height={shell.displayHeight} />
        <RuntimeHud cards={hudCards} />
        <DebugReportPanel title="Debug Report" report={debugReport} />
        <InspectorTabs pkg={shell.loadedPackage} />
      </section>
    </main>
  );
}
