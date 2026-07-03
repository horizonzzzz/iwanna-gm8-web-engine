import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CanvasStage } from './CanvasStage';

describe('CanvasStage', () => {
  it('keeps the canvas at the game display size instead of stretching to the page width', () => {
    render(<CanvasStage error={null} width={640} height={480} />);

    const canvas = document.querySelector('#room-canvas') as HTMLCanvasElement | null;

    expect(canvas).not.toBeNull();
    expect(canvas).toHaveAttribute('width', '640');
    expect(canvas).toHaveAttribute('height', '480');
    expect(canvas).toHaveClass('max-w-full');
    expect(canvas).not.toHaveClass('w-full');
  });
});
