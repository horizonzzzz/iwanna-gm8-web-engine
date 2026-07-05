import { useEffect, useMemo, useRef, useState } from 'react';

export type KeyboardInputState = {
  left: boolean;
  right: boolean;
  jump: boolean;
  restart: boolean;
  keysHeld: number[];
  keysPressed: number[];
  keysReleased: number[];
  clearEdgeKeys: () => void;
};

function keyToVirtualKey(key: string): number | null {
  switch (key) {
    case 'ArrowLeft':
      return 0x25;
    case 'ArrowUp':
      return 0x26;
    case 'ArrowRight':
      return 0x27;
    case 'ArrowDown':
      return 0x28;
    case 'Shift':
      return 0x10;
    case ' ':
    case 'Spacebar':
      return 0x20;
    case 'Enter':
      return 0x0d;
    case 'Escape':
      return 0x1b;
    default:
      return key.length === 1 ? key.toUpperCase().charCodeAt(0) : null;
  }
}

export function useKeyboardInput(): KeyboardInputState {
  const heldRef = useRef<Set<number>>(new Set());
  const [held, setHeld] = useState<Set<number>>(new Set());
  const [pressed, setPressed] = useState<Set<number>>(new Set());
  const [released, setReleased] = useState<Set<number>>(new Set());

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent): void {
      const vk = keyToVirtualKey(event.key);
      if (vk == null) {
        return;
      }
      if (event.repeat || heldRef.current.has(vk)) {
        return;
      }
      const nextHeld = new Set(heldRef.current);
      nextHeld.add(vk);
      heldRef.current = nextHeld;
      setHeld(nextHeld);
      setPressed((current) => {
        if (current.has(vk)) {
          return current;
        }
        const next = new Set(current);
        next.add(vk);
        return next;
      });
    }

    function handleKeyUp(event: KeyboardEvent): void {
      const vk = keyToVirtualKey(event.key);
      if (vk == null) {
        return;
      }
      if (!heldRef.current.has(vk)) {
        return;
      }
      const nextHeld = new Set(heldRef.current);
      nextHeld.delete(vk);
      heldRef.current = nextHeld;
      setHeld(nextHeld);
      setReleased((current) => {
        if (current.has(vk)) {
          return current;
        }
        const next = new Set(current);
        next.add(vk);
        return next;
      });
    }

    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, []);

  return useMemo(
    () => ({
      left: held.has(0x25) || held.has(0x41),
      right: held.has(0x27) || held.has(0x44),
      jump: false,
      restart: false,
      keysHeld: [...held],
      keysPressed: [...pressed],
      keysReleased: [...released],
      clearEdgeKeys: () => {
        setPressed((current) => current.size === 0 ? current : new Set());
        setReleased((current) => current.size === 0 ? current : new Set());
      },
    }),
    [held, pressed, released]
  );
}
