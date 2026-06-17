import { act, fireEvent, renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useKeyboardInput } from './useKeyboardInput';

describe('useKeyboardInput', () => {
  it('reports physical R as a raw key without semantic restart', () => {
    const { result } = renderHook(() => useKeyboardInput());

    act(() => {
      fireEvent.keyDown(window, { key: 'r' });
    });

    expect(result.current.restart).toBe(false);
    expect(result.current.keysHeld).toContain(0x52);
    expect(result.current.keysPressed).toContain(0x52);
  });
});
