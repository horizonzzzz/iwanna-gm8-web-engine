export type FixedStepLoopOptions = {
  stepMs?: number;
  maxAccumulatedMs?: number;
  now?: () => number;
};

export type FixedStepLoopCallbacks = {
  onStep: (stepMs: number) => void;
};

export class FixedStepLoop {
  readonly stepMs: number;
  readonly maxAccumulatedMs: number;

  private readonly now: () => number;
  private readonly onStep: (stepMs: number) => void;
  private accumulator = 0;
  private lastTimestamp: number | null = null;
  private paused = true;

  constructor(callbacks: FixedStepLoopCallbacks, options: FixedStepLoopOptions = {}) {
    this.onStep = callbacks.onStep;
    this.stepMs = options.stepMs ?? 1000 / 60;
    this.maxAccumulatedMs = options.maxAccumulatedMs ?? this.stepMs * 8;
    this.now = options.now ?? (() => performance.now());
  }

  get isPaused(): boolean {
    return this.paused;
  }

  resume(): void {
    this.paused = false;
    this.lastTimestamp = this.now();
  }

  pause(): void {
    this.paused = true;
    this.lastTimestamp = null;
    this.accumulator = 0;
  }

  reset(): void {
    this.accumulator = 0;
    this.lastTimestamp = this.paused ? null : this.now();
  }

  update(timestamp = this.now()): number {
    if (this.paused) {
      this.lastTimestamp = timestamp;
      return 0;
    }

    if (this.lastTimestamp == null) {
      this.lastTimestamp = timestamp;
      return 0;
    }

    const delta = Math.max(0, timestamp - this.lastTimestamp);
    this.lastTimestamp = timestamp;
    this.accumulator = Math.min(this.accumulator + delta, this.maxAccumulatedMs);

    let steps = 0;
    while (this.accumulator >= this.stepMs) {
      this.onStep(this.stepMs);
      this.accumulator -= this.stepMs;
      steps += 1;
    }

    return steps;
  }

  tick(steps = 1): void {
    for (let index = 0; index < steps; index += 1) {
      this.onStep(this.stepMs);
    }
  }
}
