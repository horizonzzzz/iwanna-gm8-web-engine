import { forwardRef } from 'react';

type CanvasStageProps = {
  error: string | null;
};

export const CanvasStage = forwardRef<HTMLCanvasElement, CanvasStageProps>(
  function CanvasStage({ error }, ref) {
    return (
      <section className="rounded border border-slate-800 bg-slate-950/80 p-3">
        <canvas
          id="room-canvas"
          ref={ref}
          width={960}
          height={540}
          className="block h-auto w-full rounded border border-slate-800 bg-slate-950"
        />
        {error ? <p className="mt-3 text-sm text-rose-300">{error}</p> : null}
      </section>
    );
  }
);
