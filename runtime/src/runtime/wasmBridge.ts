import type { RuntimePackage } from '../types';

export type WasmRuntimeBridgeSnapshot = {
  tick: number;
  roomId: number | null;
  diagnostics: string[];
};

export type WasmRuntimeBridge = {
  backend: 'opengmk-wasm';
  boot: (pkg: RuntimePackage) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  tick: (frames?: number) => Promise<WasmRuntimeBridgeSnapshot> | WasmRuntimeBridgeSnapshot;
  diagnostics: () => Promise<string[]> | string[];
};

export type WasmRuntimeBridgeModule = {
  initRuntimeHost: () => Promise<WasmRuntimeBridge> | WasmRuntimeBridge;
};

function isFunction(value: unknown): value is (...args: unknown[]) => unknown {
  return typeof value === 'function';
}

export function isWasmRuntimeBridge(value: unknown): value is WasmRuntimeBridge {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Partial<WasmRuntimeBridge>;
  return candidate.backend === 'opengmk-wasm'
    && isFunction(candidate.boot)
    && isFunction(candidate.tick)
    && isFunction(candidate.diagnostics);
}

export async function loadWasmRuntimeBridge(
  loader: () => Promise<unknown>
): Promise<WasmRuntimeBridge> {
  const loaded = await loader();
  if (!loaded || typeof loaded !== 'object' || !isFunction((loaded as Partial<WasmRuntimeBridgeModule>).initRuntimeHost)) {
    throw new Error('WASM bridge module is missing initRuntimeHost()');
  }

  const bridge = await (loaded as WasmRuntimeBridgeModule).initRuntimeHost();
  if (!isWasmRuntimeBridge(bridge)) {
    throw new Error('WASM bridge initRuntimeHost() returned an invalid bridge');
  }

  return bridge;
}

export function describeWasmBridgeAvailability(bridge: WasmRuntimeBridge | null, error: unknown): string {
  if (bridge) {
    return 'WASM bridge available; shell remains on transitional TS execution until core wiring is complete.';
  }

  if (error instanceof Error) {
    return `WASM bridge unavailable: ${error.message}`;
  }

  if (error != null) {
    return `WASM bridge unavailable: ${String(error)}`;
  }

  return 'No WASM bridge configured; shell is using the transitional TS runtime.';
}
