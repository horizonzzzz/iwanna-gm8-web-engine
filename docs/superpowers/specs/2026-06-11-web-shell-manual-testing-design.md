# Web Shell Manual Testing Design

## Purpose

The runtime web shell should become the manual testing companion to CLI runtime diagnostics.

The CLI `runtime-diagnostics` path remains the quantitative analysis surface: it runs controlled tick windows, ranks unsupported blockers, emits runtime events, and optionally records player traces. The web shell should instead optimize for interactive hand testing: load a package, drive the browser-hosted runtime, watch the room, validate input feel, and quickly understand the current runtime state without scanning dense diagnostics output.

## Current Problem

The current shell was built early as a combined package inspector, static room viewer, runtime telemetry surface, and diagnostics dump. It now shows too much information at once:

- package controls and runtime controls are mixed into a sidebar
- runtime player state is rendered as a long text row
- diagnostics and timing details compete with the playable canvas
- package inspectors remain prominent even when the developer is trying to hand-test gameplay
- CLI trace concepts such as player state, runtime events, and blocker-oriented diagnostics are not reflected as a focused manual testing view

This makes the browser shell useful but noisy. The shell should stop trying to be the main quantitative trace tool.

## Design Direction

Use a "manual testing cockpit" as the default shell mode.

Default information priority:

1. playable canvas
2. load / room / pause / reset controls
3. current room and tick
4. player position, velocity, alive state, grounded state, and jump phase
5. browser-forwarded input state and jump edge state
6. recent high-value runtime events
7. compact diagnostics health summary
8. frame budget summary
9. detailed diagnostics, tick phases, frame timing, and package inspection in collapsed debug panels

This keeps the web shell aligned with CLI trace without duplicating it. The CLI answers "what happened over a controlled run"; the web shell answers "what am I seeing while I manually play this room".

## Layout

### Main Stage

The canvas becomes the dominant page element. It should be visually and spatially treated as the primary workspace.

The stage should preserve the current `#room-canvas` id.

### Top Control Bar

Move package and runtime controls into a compact top bar:

- package path input
- `Load Package`
- room select
- `Pause` / `Resume`
- `Reset`
- execution path status

The top bar should avoid consuming vertical space needed by the canvas and should keep the hand-test loop visible.

### Manual Test HUD

Render a concise runtime HUD near the canvas. It should include:

- current runtime status
- current room
- current tick
- player summary
- input summary
- recent runtime event summary
- diagnostics summary
- frame budget summary

Keep these existing semantic ids available for tests and external smoke checks:

- `#runtime-status`
- `#runtime-room`
- `#runtime-tick`
- `#runtime-player`
- `#runtime-diagnostics`

The text behind those ids can become shorter and more structured, but the ids should continue to represent the same concepts.

### Debug Drawers

Move lower-priority detail into collapsed panels:

- Diagnostics: recent full diagnostic lines
- Performance: input, tick, snapshot, frame, render, runtime, command count, skipped intervals
- Tick phases: runtime-core phase timings
- Package: manifest and analysis summary
- Inspectors: rooms, objects, and script slices

These panels are for investigation after a visible hand-test problem appears. They should not dominate the default view.

## UI Module Boundaries

Keep the current plain TypeScript DOM-rendering approach. Do not introduce React or a new UI framework for this change.

Recommended runtime UI modules:

- `runtime/src/ui/shell.ts`
  Owns shell state, package loading, event binding, WASM session lifecycle, pause/resume, reset, room selection, and orchestration.

- `runtime/src/ui/hud.ts`
  Renders the manual testing HUD. It owns the short status, room, tick, player, input, event, diagnostics, and frame budget presentation.

- `runtime/src/ui/debugPanels.ts`
  Renders collapsed diagnostics, performance, tick-phase, package, and inspector panels.

- `runtime/src/ui/traceView.ts`
  Converts bridge snapshots and shell timing data into a browser-local manual trace summary. This should be a presentation adapter, not a new runtime contract.

- `runtime/src/ui/inspectors.ts`
  Keeps package inspection rendering, but callers should place it in a collapsed debug panel by default.

This split keeps `shell.ts` from continuing to grow as a mixed state manager and renderer.

## Data Model

Add a shell-side `ManualTestSnapshot` or equivalent view model derived from existing browser data:

- WASM bridge snapshot
- current room label
- player snapshot
- input trace snapshot
- recent diagnostics strings
- shell frame timing
- recent selected runtime event strings

Do not change the Rust runtime, runtime-core, or WASM bridge contract for the first implementation unless the existing string diagnostics prove insufficient.

Initial runtime event extraction can be string-based from diagnostics. The relevant event codes are:

- `runtime-room-changed`
- `runtime-room-restart-requested`
- `runtime-player-died`
- `runtime-instance-created`
- `runtime-instance-destroyed`

If later work needs richer event filtering or rendering, add structured bridge data in a separate runtime-web change with Rust tests.

## Diagnostics Policy

Default diagnostics should be a health summary, not a dump:

- show `Diagnostics: none` when clean
- show a bounded recent count when non-empty
- surface the newest high-value runtime event separately from generic diagnostic noise
- keep full recent diagnostics in the collapsed Diagnostics panel

Unsupported runtime diagnostics still matter, but the web shell should not replace CLI blocker ranking. If unsupported diagnostics appear during hand testing, the shell should make the condition visible and then point the developer toward CLI `runtime-diagnostics` for quantitative ranking.

## Performance Policy

Default performance display should answer whether hand testing is trustworthy:

- show total frame time
- show skipped auto-tick intervals
- show command count only if useful and compact

Detailed timings should stay in the collapsed Performance and Tick Phases panels:

- input
- tick
- snapshot
- frame
- canvas render
- runtime total
- runtime-core phase timings

This keeps performance information available without turning the default shell into a profiler.

## Static Viewer And Failure States

The shell must continue to degrade explicitly:

- If WASM is available and boot succeeds, the shell enters manual runtime testing mode.
- If WASM is missing or boot fails, the shell falls back to static viewer mode and clearly marks the execution path.
- If package load fails, clear runtime HUD state, keep the package path input usable, and show the load error.
- If a runtime tick fails, stop auto-ticking and show the tick failure in status plus diagnostics detail.

Static viewer mode should remain useful for package inspection, but it should not look like a playable runtime.

## Testing Strategy

Update frontend unit tests in `runtime/src/main.test.ts` to cover:

- default shell structure
- package load into the new HUD
- preserved smoke-test ids
- collapsed debug panels
- WASM fallback to static viewer
- pause/resume behavior
- reset behavior
- room selection behavior
- bounded diagnostics display
- performance summary and detailed performance panel

Update browser smoke tests in `runtime/tests/browser/runtime-shell.spec.ts` to keep coverage of:

- WASM boot status
- current room label
- current tick advancement
- player summary availability
- diagnostics summary availability
- room selection into a playable room
- pause/resume tick behavior

Expected verification after implementation:

```powershell
npm --prefix runtime test
npm --prefix runtime run test:browser
```

`npm --prefix runtime run test:browser` depends on local WASM and sample package prerequisites. If those are missing in the local environment, record that limitation instead of treating it as a code failure.

Rust tests are not required for the first UI-only implementation. If implementation changes `iwm-runtime-web` bridge data, then add relevant Rust tests and run the narrow Rust test set plus `cargo test`.

## Documentation Impact

This change is shell UX and telemetry presentation work. It does not change parser output, runtime-core semantics, or the package contract.

Update current docs only if implementation changes the actual workflow or bridge contract:

- update `README.md` if shell usage or visible controls change materially
- update `docs/notes/runtime-wasm-gap-analysis.md` if runtime telemetry behavior changes what browser debugging can actually observe
- update `docs/notes/package-format-v1-runtime.md` only if package or bridge contract changes

## Non-Goals

This design does not:

- add new runtime semantics
- expand the TypeScript gameplay fallback
- replace CLI `runtime-diagnostics`
- introduce a new frontend framework
- make package inspectors the default shell focus
- require structured Rust-side runtime events in the first implementation

## Success Criteria

The implementation is successful when:

- a developer can load a package and immediately hand-test gameplay without scanning dense debug text
- the canvas is visually primary
- current room, tick, player, input, diagnostics, and frame-budget status are visible at a glance
- detailed diagnostics and inspectors remain available but collapsed by default
- existing runtime smoke ids continue to work
- frontend tests verify the new default hierarchy
- CLI trace remains the recommended path for quantitative blocker and player-trace analysis
