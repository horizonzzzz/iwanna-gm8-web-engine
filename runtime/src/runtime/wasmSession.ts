import type { WasmRuntimeBridge, WasmRuntimeFrame, WasmRuntimeInputState } from './wasmBridge';
import type { WasmRuntimeBridgeSnapshot } from './wasmBridge';

export type WasmRuntimeStepResult = {
  snapshot: WasmRuntimeBridgeSnapshot;
  frame: WasmRuntimeFrame;
  timings: WasmRuntimeStepTimings;
};

export type WasmRuntimeStepTimings = {
  inputMs: number;
  tickMs: number;
  snapshotMs: number;
  frameMs: number;
  runtimeMs: number;
};

const DEFAULT_INPUT: WasmRuntimeInputState = {
  left: false,
  right: false,
  jump: false,
  jumpPressed: false,
  jumpReleased: false,
  restart: false,
  keysHeld: [],
  keysPressed: [],
  keysReleased: []
};

export class WasmRuntimeSession {
  private input: WasmRuntimeInputState = { ...DEFAULT_INPUT };
  private previousJump = false;
  private previousKeys = new Set<number>();
  private pendingJumpPressed = false;
  private pendingJumpReleased = false;
  private pendingKeyPresses = new Set<number>();
  private pendingKeyReleases = new Set<number>();

  constructor(
    private readonly bridge: WasmRuntimeBridge,
    private readonly now: () => number = () => globalThis.performance?.now() ?? Date.now()
  ) {}

  setInputState(
    snapshot: Pick<WasmRuntimeInputState, 'left' | 'right' | 'jump' | 'restart'>
      & { keysHeld?: number[]; keysPressed?: number[]; keysReleased?: number[] }
  ): void {
    const jumpPressed = snapshot.jump && !this.previousJump;
    const jumpReleased = !snapshot.jump && this.previousJump;
    const heldKeys = new Set(snapshot.keysHeld ?? []);
    const keysPressed = new Set([
      ...[...heldKeys].filter((key) => !this.previousKeys.has(key)),
      ...(snapshot.keysPressed ?? [])
    ]);
    const keysReleased = new Set([
      ...[...this.previousKeys].filter((key) => !heldKeys.has(key)),
      ...(snapshot.keysReleased ?? [])
    ]);

    if (jumpPressed) {
      this.pendingJumpPressed = true;
    }
    if (jumpReleased) {
      this.pendingJumpReleased = true;
    }
    for (const key of keysPressed) {
      this.pendingKeyPresses.add(key);
    }
    for (const key of keysReleased) {
      this.pendingKeyReleases.add(key);
    }

    this.input = {
      left: snapshot.left,
      right: snapshot.right,
      jump: snapshot.jump,
      jumpPressed: this.pendingJumpPressed,
      jumpReleased: this.pendingJumpReleased,
      restart: snapshot.restart,
      keysHeld: [...heldKeys],
      keysPressed: [...this.pendingKeyPresses],
      keysReleased: [...this.pendingKeyReleases]
    };

    this.previousJump = snapshot.jump;
    this.previousKeys = heldKeys;
  }

  async stepOnce(): Promise<WasmRuntimeStepResult> {
    const input = { ...this.input };
    const runtimeStart = this.now();
    const inputStart = this.now();
    await this.bridge.setInput(input);
    const inputMs = this.now() - inputStart;
    const tickStart = this.now();
    await this.bridge.tick(1);
    const tickMs = this.now() - tickStart;
    const snapshotStart = this.now();
    const snapshot = await this.bridge.snapshot();
    const snapshotMs = this.now() - snapshotStart;
    const frameStart = this.now();
    const frame = await this.bridge.frame();
    const frameMs = this.now() - frameStart;
    const runtimeMs = this.now() - runtimeStart;
    this.pendingJumpPressed = false;
    this.pendingJumpReleased = false;
    this.pendingKeyPresses.clear();
    this.pendingKeyReleases.clear();
    this.input.jumpPressed = false;
    this.input.jumpReleased = false;
    this.input.keysPressed = [];
    this.input.keysReleased = [];
    return {
      snapshot,
      frame,
      timings: {
        inputMs,
        tickMs,
        snapshotMs,
        frameMs,
        runtimeMs
      }
    };
  }
}
