type RoomOption = {
  id: number;
  name: string;
};

type ControlBarProps = {
  packagePath: string;
  onPackagePathChange: (value: string) => void;
  onLoad: () => void;
  roomOptions: RoomOption[];
  selectedRoomId: number | null;
  onRoomChange: (roomId: number) => void;
  autoTickRunning: boolean;
  runtimeReady: boolean;
  onPauseToggle: () => void;
  onReset: () => void;
  backendStatus: string;
};

export function ControlBar(props: ControlBarProps): JSX.Element {
  const pauseLabel = props.runtimeReady
    ? (props.autoTickRunning ? 'Pause' : 'Resume')
    : 'Pause';

  return (
    <header className="border-b border-slate-800 bg-slate-950/95 px-6 py-4">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
        <div>
          <h1 className="text-lg font-semibold text-slate-100">IWanna Runtime Shell</h1>
          <p className="mt-1 text-sm text-slate-400">
            Manual testing cockpit for the browser-hosted runtime path.
          </p>
        </div>
        <div className="grid gap-3 md:grid-cols-[minmax(20rem,1fr)_14rem_auto_auto_auto]">
          <label className="text-sm text-slate-300">
            <span className="mb-1 block">Package</span>
            <input
              aria-label="Package"
              name="packagePath"
              value={props.packagePath}
              onChange={(event) => props.onPackagePathChange(event.target.value)}
              className="w-full rounded border border-slate-700 bg-slate-900 px-3 py-2 text-sm"
            />
          </label>
          <label className="text-sm text-slate-300">
            <span className="mb-1 block">Room</span>
            <select
              aria-label="Room"
              name="roomSelect"
              disabled={props.roomOptions.length === 0}
              value={props.selectedRoomId ?? ''}
              onChange={(event) => props.onRoomChange(Number(event.target.value))}
              className="w-full rounded border border-slate-700 bg-slate-900 px-3 py-2 text-sm disabled:opacity-50"
            >
              {props.roomOptions.length === 0 ? (
                <option value="">Load a package first</option>
              ) : (
                props.roomOptions.map((room) => (
                  <option key={room.id} value={room.id}>
                    {room.id}: {room.name}
                  </option>
                ))
              )}
            </select>
          </label>
          <button
            type="button"
            onClick={props.onLoad}
            className="rounded border border-slate-700 px-3 py-2 text-sm"
          >
            Load Package
          </button>
          <button
            type="button"
            disabled={!props.runtimeReady}
            onClick={props.onPauseToggle}
            className="rounded border border-slate-700 px-3 py-2 text-sm disabled:opacity-50"
          >
            {pauseLabel}
          </button>
          <button
            type="button"
            disabled={!props.runtimeReady}
            onClick={props.onReset}
            className="rounded border border-slate-700 px-3 py-2 text-sm disabled:opacity-50"
          >
            Reset
          </button>
        </div>
      </div>
      <p className="mt-3 text-xs text-slate-400">{props.backendStatus}</p>
    </header>
  );
}
