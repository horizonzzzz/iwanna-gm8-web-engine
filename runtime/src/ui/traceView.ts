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
