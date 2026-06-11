import { useMemo } from 'react';
import type { WasmRuntimeBridgeSnapshot } from '../../runtime/wasmBridge';
import type { RuntimePerformanceStats } from '../traceView';
import { buildDebugReport } from '../formatters/debugReport';

type UseDebugReportInput = {
  status: string;
  roomLabel: string;
  snapshot: WasmRuntimeBridgeSnapshot | null;
  performance: RuntimePerformanceStats | null;
};

export function useDebugReport(input: UseDebugReportInput): string {
  return useMemo(() => {
    if (!input.snapshot) {
      return `Status: ${input.status}\nRoom: ${input.roomLabel}\nTick: 0\n\nDiagnostics:\n- none`;
    }

    return buildDebugReport({
      mode: 'wasm',
      status: input.status,
      roomLabel: input.roomLabel,
      snapshot: input.snapshot,
      performance: input.performance,
    });
  }, [input.performance, input.roomLabel, input.snapshot, input.status]);
}
