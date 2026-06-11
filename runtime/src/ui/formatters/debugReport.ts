import type { WasmRuntimeBridgeSnapshot } from '../../runtime/wasmBridge';
import type { RuntimePerformanceStats } from '../traceView';

type BuildDebugReportInput = {
  mode: 'wasm' | 'viewer';
  status: string;
  roomLabel: string;
  snapshot: WasmRuntimeBridgeSnapshot;
  performance: RuntimePerformanceStats | null;
};

function formatMs(value: number): string {
  return value.toFixed(1);
}

function formatPhaseMs(value: number): string {
  return (value / 1_000_000).toFixed(3);
}

export function buildDebugReport(input: BuildDebugReportInput): string {
  const { snapshot, performance } = input;
  const player = snapshot.player;
  const playerLines = player
    ? [
        `- x=${player.x} y=${player.y}`,
        `- hspeed=${player.hspeed} vspeed=${player.vspeed}`,
        `- object=${player.objectName ?? 'unknown'}#${player.runtimeId ?? '?'}`,
        `- alive=${player.alive ?? true} grounded=${player.jump.grounded}`,
        `- jumpActive=${player.jump.active} hold=${player.jump.holdFrames} cut=${player.jump.cutApplied}`,
      ]
    : ['- unavailable'];

  const performanceLines = performance
    ? [
        `- total=${formatMs(performance.totalMs)}ms budget=${performance.totalMs <= 16.7 ? 'ok' : 'slow'} skipped=${performance.skippedIntervals} commands=${performance.commandCount}`,
        `- input=${formatMs(performance.inputMs)} tick=${formatMs(performance.tickMs)} snapshot=${formatMs(performance.snapshotMs)} frame=${formatMs(performance.frameMs)} render=${formatMs(performance.renderMs)} runtime=${formatMs(performance.runtimeMs)}`,
      ]
    : ['- unavailable'];

  const tickPhaseLines = snapshot.tickPhases
    ? [
        `- total=${formatPhaseMs(snapshot.tickPhases.totalNanos)}ms`,
        `- inputDiag=${formatPhaseMs(snapshot.tickPhases.inputDiagNanos)} step=${formatPhaseMs(snapshot.tickPhases.stepEventsNanos)} view=${formatPhaseMs(snapshot.tickPhases.viewSyncNanos)} player=${formatPhaseMs(snapshot.tickPhases.playerMovementNanos)}`,
        `- collision=${formatPhaseMs(snapshot.tickPhases.collisionEventsNanos)} alarms=${formatPhaseMs(snapshot.tickPhases.alarmsNanos)} keyboard=${formatPhaseMs(snapshot.tickPhases.keyboardEventsNanos)} renderSubmit=${formatPhaseMs(snapshot.tickPhases.renderSubmitNanos)}`,
      ]
    : ['- unavailable'];

  const recentEvents = snapshot.diagnostics
    .filter((item) => item.includes('runtime-'))
    .slice(-5)
    .map((item) => `- ${item}`);

  return [
    `Status: ${input.status}`,
    `Room: ${input.roomLabel}`,
    `Tick: ${snapshot.tick}`,
    '',
    'Player:',
    ...playerLines,
    '',
    'Input:',
    `- jumpKey=0x${snapshot.inputTrace.jumpButtonKey.toString(16)}`,
    `- pressed=${snapshot.inputTrace.jumpPressed} justPressed=${snapshot.inputTrace.jumpJustPressed} justReleased=${snapshot.inputTrace.jumpJustReleased}`,
    `- keys=[${snapshot.inputTrace.activeKeys.join(',')}]`,
    '',
    'Performance:',
    ...performanceLines,
    '',
    'Tick Phases:',
    ...tickPhaseLines,
    '',
    'Recent Events:',
    ...(recentEvents.length > 0 ? recentEvents : ['- none']),
    '',
    'Diagnostics:',
    ...(snapshot.diagnostics.length > 0 ? snapshot.diagnostics.map((item) => `- ${item}`) : ['- none']),
  ].join('\n');
}
