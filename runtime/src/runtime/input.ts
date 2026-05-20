import type { RuntimeInputSnapshot } from './types';

const DEFAULT_INPUT: RuntimeInputSnapshot = {
  left: false,
  right: false,
  jump: false,
  jumpPressed: false,
  jumpReleased: false,
  restart: false
};

export class RuntimeInputController {
  private current: RuntimeInputSnapshot = { ...DEFAULT_INPUT };
  private previous: RuntimeInputSnapshot = { ...DEFAULT_INPUT };

  setKeyState(action: 'left' | 'right' | 'jump' | 'restart', pressed: boolean): void {
    this.current[action] = pressed;
  }

  setSnapshot(snapshot: Pick<RuntimeInputSnapshot, 'left' | 'right' | 'jump' | 'restart'>): void {
    this.current.left = snapshot.left;
    this.current.right = snapshot.right;
    this.current.jump = snapshot.jump;
    this.current.restart = snapshot.restart;
  }

  sample(): RuntimeInputSnapshot {
    const snapshot: RuntimeInputSnapshot = {
      left: this.current.left,
      right: this.current.right,
      jump: this.current.jump,
      restart: this.current.restart,
      jumpPressed: this.current.jump && !this.previous.jump,
      jumpReleased: !this.current.jump && this.previous.jump
    };

    this.previous = { ...snapshot };
    this.current.jumpPressed = false;
    this.current.jumpReleased = false;

    return snapshot;
  }

  reset(): void {
    this.current = { ...DEFAULT_INPUT };
    this.previous = { ...DEFAULT_INPUT };
  }
}
