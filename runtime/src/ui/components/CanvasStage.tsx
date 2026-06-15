import { forwardRef } from 'react';

type CanvasStageProps = {
  error: string | null;
  /** Initial canvas width in CSS pixels. */
  width?: number;
  /** Initial canvas height in CSS pixels. */
  height?: number;
};

export const CanvasStage = forwardRef<HTMLCanvasElement, CanvasStageProps>(
  function CanvasStage({ error, width = 800, height = 600 }, ref) {
    return (
      <section className="rounded border border-slate-800 bg-slate-950/80 p-3">
        <canvas
          id="room-canvas"
          ref={ref}
          width={width}
          height={height}
          className="block h-auto w-full rounded border border-slate-800 bg-slate-950"
        />
        {error ? <p className="mt-3 text-sm text-rose-300">{error}</p> : null}
      </section>
    );
  }
);
