import type { WasmRuntimeBridgeSnapshot, WasmRuntimeTickPhases } from '../runtime/wasmBridge';

export type RuntimePerformanceStats = {
  inputMs: number;
  tickMs: number;
  snapshotMs: number;
  frameMs: number;
  runtimeMs: number;
  renderMs: number;
  totalMs: number;
  commandCount: number;
  skippedIntervals: number;
};

export type ManualTestSnapshot = {
  mode: 'wasm' | 'viewer';
  status: string;
  roomLabel: string;
  tickLabel: string;
  playerSummary: string;
  inputSummary: string;
  diagnosticsSummary: string;
  frameBudgetSummary: string;
  recentEvents: string[];
  diagnostics: string[];
  performance: RuntimePerformanceStats | null;
  tickPhases: WasmRuntimeTickPhases | null;
};

type BuildManualTestSnapshotInput = {
  mode: 'wasm' | 'viewer';
  status: string;
  roomLabel: string;
  snapshot: WasmRuntimeBridgeSnapshot;
  performance?: RuntimePerformanceStats | null;
};

const RUNTIME_EVENT_CODES = [
  'runtime-room-changed',
  'runtime-room-restart-requested',
  'runtime-player-died',
  'runtime-instance-created',
  'runtime-instance-destroyed',
];

function formatNumber(value: number): string {
  if (Number.isInteger(value)) {
    return String(value);
  }
  return value.toFixed(3).replace(/0+$/, '').replace(/\.$/, '');
}

function extractEventCode(diagnostic: string): string | null {
  return RUNTIME_EVENT_CODES.find((code) => diagnostic.includes(code)) ?? null;
}

export function extractRuntimeEvents(diagnostics: string[]): string[] {
  return diagnostics.filter((diagnostic) => extractEventCode(diagnostic) != null).slice(-5);
}

export function formatRuntimePlayer(snapshot: WasmRuntimeBridgeSnapshot): string {
  if (!snapshot.player) {
    return 'Player: unavailable';
  }

  const player = snapshot.player;
  const objectLabel = player.objectName
    ? `${player.objectName}#${player.runtimeId ?? '?'}`
    : 'unknown';

  return [
    `Player: x=${formatNumber(player.x)}`,
    `y=${formatNumber(player.y)}`,
    `hspeed=${formatNumber(player.hspeed)}`,
    `vspeed=${formatNumber(player.vspeed)}`,
    `object=${objectLabel}`,
    `alive=${player.alive ?? true}`,
    `grounded=${player.jump.grounded}`,
    `jumpActive=${player.jump.active}`,
    `hold=${player.jump.holdFrames}`,
    `cut=${player.jump.cutApplied}`,
  ].join(' ');
}

export function formatInputSummary(snapshot: WasmRuntimeBridgeSnapshot): string {
  const trace = snapshot.inputTrace;
  return [
    `Input: jumpKey=0x${trace.jumpButtonKey.toString(16)}`,
    `pressed=${trace.jumpPressed}`,
    `justPressed=${trace.jumpJustPressed}`,
    `justReleased=${trace.jumpJustReleased}`,
    `keys=[${trace.activeKeys.join(',')}]`,
  ].join(' ');
}

export function formatDiagnosticsSummary(diagnostics: string[]): string {
  if (diagnostics.length === 0) {
    return 'Diagnostics: none';
  }

  const latestEventCode = extractRuntimeEvents(diagnostics)
    .map((event) => extractEventCode(event))
    .filter((event): event is string => event != null)
    .at(-1);

  if (latestEventCode) {
    return `Diagnostics: ${diagnostics.length} recent, latest event ${latestEventCode}`;
  }

  return `Diagnostics: ${diagnostics.length} recent`;
}

export function formatFrameBudgetSummary(performance: RuntimePerformanceStats | null | undefined): string {
  if (!performance) {
    return 'Frame: unavailable';
  }

  const budget = performance.totalMs <= 16.7 ? 'ok' : 'slow';
  return [
    `Frame: ${performance.totalMs.toFixed(1)}ms`,
    budget,
    `skipped=${performance.skippedIntervals}`,
    `commands=${performance.commandCount}`,
  ].join(' ');
}

export function buildManualTestSnapshot(input: BuildManualTestSnapshotInput): ManualTestSnapshot {
  return {
    mode: input.mode,
    status: input.status,
    roomLabel: input.roomLabel,
    tickLabel: `Tick: ${input.snapshot.tick}`,
    playerSummary: formatRuntimePlayer(input.snapshot),
    inputSummary: formatInputSummary(input.snapshot),
    diagnosticsSummary: formatDiagnosticsSummary(input.snapshot.diagnostics),
    frameBudgetSummary: formatFrameBudgetSummary(input.performance),
    recentEvents: extractRuntimeEvents(input.snapshot.diagnostics),
    diagnostics: input.snapshot.diagnostics,
    performance: input.performance ?? null,
    tickPhases: input.snapshot.tickPhases ?? null,
  };
}
