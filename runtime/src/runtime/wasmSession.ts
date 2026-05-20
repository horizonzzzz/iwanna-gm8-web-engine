import type { WasmRuntimeBridge, WasmRuntimeFrame, WasmRuntimeInputState } from './wasmBridge';

const DEFAULT_INPUT: WasmRuntimeInputState = {
  left: false,
  right: false,
  jump: false,
  jumpPressed: false,
  jumpReleased: false,
  restart: false
};

export class WasmRuntimeSession {
  private input: WasmRuntimeInputState = { ...DEFAULT_INPUT };
  private previousJump = false;

  constructor(private readonly bridge: WasmRuntimeBridge) {}

  setInputState(snapshot: Pick<WasmRuntimeInputState, 'left' | 'right' | 'jump' | 'restart'>): void {
    const jumpPressed = snapshot.jump && !this.previousJump;
    const jumpReleased = !snapshot.jump && this.previousJump;

    this.input = {
      left: snapshot.left,
      right: snapshot.right,
      jump: snapshot.jump,
      jumpPressed,
      jumpReleased,
      restart: snapshot.restart
    };
  }

  async stepOnce(): Promise<WasmRuntimeFrame> {
    const input = { ...this.input };
    await this.bridge.setInput(input);
    await this.bridge.tick(1);
    const frame = await this.bridge.frame();
    this.previousJump = input.jump;
    this.input.jumpPressed = false;
    this.input.jumpReleased = false;
    return frame;
  }
}
