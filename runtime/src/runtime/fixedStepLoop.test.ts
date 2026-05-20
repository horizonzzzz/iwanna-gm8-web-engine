import { describe, expect, it, vi } from 'vitest';
import { FixedStepLoop } from './fixedStepLoop';

describe('FixedStepLoop', () => {
  it('ticks deterministically in manual mode', () => {
    const onStep = vi.fn();
    const loop = new FixedStepLoop({ onStep }, { stepMs: 16 });
    loop.tick(3);
    expect(onStep).toHaveBeenCalledTimes(3);
    expect(onStep).toHaveBeenNthCalledWith(1, 16);
  });

  it('does not step while paused during update', () => {
    const onStep = vi.fn();
    const loop = new FixedStepLoop({ onStep }, { stepMs: 16, now: () => 0 });
    loop.update(32);
    expect(onStep).not.toHaveBeenCalled();
  });
});
