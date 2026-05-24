import type { WasmRuntimeBridge, WasmRuntimeFrame, WasmRuntimeInputState } from './wasmBridge';
import type { WasmRuntimeBridgeSnapshot } from './wasmBridge';

export type WasmRuntimeStepResult = {
  snapshot: WasmRuntimeBridgeSnapshot;
  frame: WasmRuntimeFrame;
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

  constructor(private readonly bridge: WasmRuntimeBridge) {}

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
    await this.bridge.setInput(input);
    await this.bridge.tick(1);
    const snapshot = await this.bridge.snapshot();
    const frame = await this.bridge.frame();
    this.pendingJumpPressed = false;
    this.pendingJumpReleased = false;
    this.pendingKeyPresses.clear();
    this.pendingKeyReleases.clear();
    this.input.jumpPressed = false;
    this.input.jumpReleased = false;
    this.input.keysPressed = [];
    this.input.keysReleased = [];
    return { snapshot, frame };
  }
}
