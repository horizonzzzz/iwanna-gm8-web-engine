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

  it('does not re-render when clearing already-empty edge keys', () => {
    let renders = 0;
    const { result } = renderHook(() => {
      renders += 1;
      return useKeyboardInput();
    });

    expect(renders).toBe(1);

    act(() => {
      result.current.clearEdgeKeys();
    });

    expect(renders).toBe(1);
    expect(result.current.keysPressed).toEqual([]);
    expect(result.current.keysReleased).toEqual([]);
  });

  it('clears pending edge keys when they exist', () => {
    const { result } = renderHook(() => useKeyboardInput());

    act(() => {
      fireEvent.keyDown(window, { key: 'Shift' });
    });
    expect(result.current.keysPressed).toContain(0x10);

    act(() => {
      result.current.clearEdgeKeys();
    });

    expect(result.current.keysHeld).toContain(0x10);
    expect(result.current.keysPressed).toEqual([]);
    expect(result.current.keysReleased).toEqual([]);
  });

  it('does not report repeated keydown events as new edge presses', () => {
    const { result } = renderHook(() => useKeyboardInput());

    act(() => {
      fireEvent.keyDown(window, { key: 'Shift' });
    });
    expect(result.current.keysPressed).toEqual([0x10]);

    act(() => {
      result.current.clearEdgeKeys();
    });
    expect(result.current.keysHeld).toEqual([0x10]);
    expect(result.current.keysPressed).toEqual([]);

    act(() => {
      fireEvent.keyDown(window, { key: 'Shift', repeat: true });
    });

    expect(result.current.keysHeld).toEqual([0x10]);
    expect(result.current.keysPressed).toEqual([]);
  });
});
