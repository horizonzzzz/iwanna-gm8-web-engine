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
    let snapshot: WasmRuntimeBridgeSnapshot;
    let frame: WasmRuntimeFrame;
    let inputMs = 0;
    let tickMs = 0;
    let snapshotMs = 0;
    let frameMs = 0;

    if (this.bridge.step) {
      const stepStart = this.now();
      const result = await this.bridge.step(input);
      const stepMs = this.now() - stepStart;
      inputMs = stepMs;
      tickMs = 0;
      snapshotMs = 0;
      frameMs = 0;
      snapshot = result.snapshot;
      frame = result.frame;
    } else {
      const inputStart = this.now();
      await this.bridge.setInput(input);
      inputMs = this.now() - inputStart;
      const tickStart = this.now();
      await this.bridge.tick(1);
      tickMs = this.now() - tickStart;
      const snapshotStart = this.now();
      snapshot = await this.bridge.snapshot();
      snapshotMs = this.now() - snapshotStart;
      const frameStart = this.now();
      frame = await this.bridge.frame();
      frameMs = this.now() - frameStart;
    }
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
