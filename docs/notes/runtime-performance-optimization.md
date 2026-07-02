# Runtime Performance Optimization Notes

> **Current status note:** This document records the performance findings from
> the current WASM runtime profiling pass. Keep it focused on actionable runtime
> and host architecture work, not general gameplay fidelity gaps.

This note complements `docs/notes/runtime-wasm-gap-analysis.md`. The gap
analysis tracks whether the game can run correctly; this note tracks whether the
same runtime path can stay smooth enough to feel close to the original EXE.

## Current Baseline

Local room151 browser measurements after syncing a release WASM build:

- command path: `cargo build -p iwm-runtime-web --release --target wasm32-unknown-unknown`
- sync path: `npm --prefix runtime run sync:wasm`
- smoke path: `npx playwright test --grep room151`
- `stepMs`: average `5.30ms`, p95 `8.10ms`, max `11.20ms`
- `renderMs`: average `1.56ms`, p95 `2.70ms`, max `5.80ms`
- `totalMs`: average `6.87ms`, p95 `10.20ms`, max `12.80ms`
- final frame command count: `783`

The earlier debug-WASM room151 measurement was much slower:

- `totalMs` average `41.23ms`
- `totalMs` p95 `61.30ms`

Interpretation:

- release WASM is mandatory for browser performance testing
- room151 is currently inside a 60 Hz frame budget on this machine
- debug WASM numbers are useful only for correctness debugging, not smoothness
- future larger rooms can still stall because several runtime paths scale by
  repeatedly scanning or rebuilding data structures

## Current Bottleneck Ranking

1. Build mode was the first fixed cost. Release WASM is now the default sync
   path and should remain the default for browser smoke tests.
2. The JSON bridge is still a medium-term fixed tax. It creates encode, decode,
   parse, stringify, and GC pressure at every tick/frame boundary. Treat this as
   a separate bridge-contract project.
3. Runtime-core indexing and allocation churn is the best next core target:
   stronger spatial indexes, scratch-buffer reuse, and avoiding per-tick
   `HashMap` / `Vec` rebuilds.
4. Browser canvas rendering is measurable but not the primary room151 bottleneck
   in release mode.
5. The browser tick loop matters for perceived smoothness, but it does not
   explain the earlier debug-WASM stalls by itself.

## Runtime-Core Refactors

### 1. Add a Tick Context

Introduce a runtime-owned tick context, for example `RuntimeTickContext`, that is
cleared and reused for one runtime tick. It should hold phase-local indexes,
scratch vectors, and temporary views instead of letting each subsystem allocate
its own short-lived structures.

Candidate contents:

- `RuntimeScratch` reusable `Vec`s for dispatch owners, collision candidates,
  spatial cells, draw commands, diagnostics, and pending lifecycle events
- a room-state epoch so cached indexes are invalidated on room change or reset
- an instance epoch or dirty list so movement, creation, and destruction can
  update indexes without forcing every query to rebuild from scratch
- per-tick object membership views for object-target queries such as
  `instance_number`, `with(object)`, `distance_to_object`, and collision helpers

The important constraint is correctness: do not share a stale spatial index
across phases that can move, create, or destroy instances. Either rebuild once at
well-defined phase boundaries, or use a dirty overlay that all same-tick queries
consult before falling back to the base index.

### 2. Make Spatial Indexing Denser

The current architecture already has spatial indexing on the collision hot path,
but the next version should be less dependent on nested hash maps and repeated
candidate deduplication.

Practical sequence:

1. Keep the existing spatial-index behavior, but move its allocations into the
   tick context and reuse buckets with `clear()` so capacity survives across
   ticks.
2. Replace object-id hash lookup on hot paths with package-load ordinals:
   `object_id -> object_ord` once, then `Vec<ObjectSpatialBuckets>` at runtime.
3. Pack grid cells into a small key, for example `(cell_x, cell_y) -> i64`, and
   reuse a scratch list of nearby cell keys per query.
4. Avoid `Vec::contains` or per-query `HashSet` for candidate deduplication.
   Use `candidate_mark: Vec<u32>` indexed by runtime instance slot plus a
   monotonic `mark_epoch`. When the epoch overflows, clear the mark array.
5. Cache object inheritance expansion, such as "all children of collision target
   object X", at package/runtime load instead of recomputing it during collision
   dispatch.

This can remain compatible with the current package model. It does not require a
new parser format.

### 3. Precompute Event Dispatch Tables

Build dispatch tables when the runtime package is loaded:

- `object_ord -> event_tag -> lowered_entry_indices`
- `object_ord -> collision_target_ord -> lowered_entry_indices`
- `object_ord -> parent_chain`
- `object_ord -> child_object_ords`

Runtime event loops should hold indices into the package-owned lowered-entry
storage instead of cloning lowered entries or searching event lists every tick.

This matters for rooms with many instances that share the same object type. The
dispatch question is mostly static; only the live instance set changes every
tick.

### 4. Reuse Scratch Buffers

Add persistent scratch storage to `RuntimeCore` or to a contained runtime context.
Clear vectors between phases, but keep their capacity.

Good scratch-buffer candidates:

- collision and place-meeting candidate lists
- object-target instance lists
- pending created/destroyed runtime ids
- dispatch owner lists
- temporary draw-command lists
- nearby spatial cell lists
- per-phase diagnostics buffers

Use high-water caps where needed. For example, if a vector grows because of a
pathological room, allow it to shrink after room change or after several ticks
below a lower threshold. This avoids trading CPU stalls for unbounded memory
retention.

### 5. Replace Per-Tick Update HashMaps With Sparse Overlays

Same-tick update visibility is required for correctness, but a generic
`HashMap<usize, RuntimeInstance>` style overlay is expensive when it is rebuilt
frequently.

Prefer a sparse overlay:

- `dirty_indices: Vec<usize>`
- `overlay_slots: Vec<Option<PendingInstanceDelta>>` or equivalent sparse-set
  storage indexed by runtime instance slot
- per-field deltas where possible, instead of cloning full `RuntimeInstance`
  values
- clear only dirty slots after the phase, using `dirty_indices`

This preserves same-tick read-after-write behavior while making the common case
cheap.

### 6. Cache Static Render Data

Render-frame construction still has room to avoid repeated work even though
canvas rendering is not the current room151 bottleneck.

Cache per-room static data:

- visible background commands
- static tile commands or tile spatial buckets
- resource lookup tables for sprite/background/font ids
- room-level metadata needed by the renderer

Invalidate this cache on room change, package reload, or when runtime semantics
eventually support mutable tile/background state. Dynamic instances and draw
events should stay per-frame.

## JSON Bridge Work

Moving the bridge from JSON strings to a binary buffer can remove a real fixed
cost:

- Rust-side `serde_json::to_string`
- JS-side `TextDecoder`
- JS-side `JSON.parse`
- JS object allocation and GC pressure
- repeated command array materialization

It will not solve every smoothness issue:

- browser timer scheduling can still slip
- main-thread work can still be interrupted by layout, input, devtools, or other
  page work
- tab throttling and power policy still exist
- audio autoplay and timing policy still differ from native EXE behavior
- runtime-core algorithms can still be too expensive if they scan or allocate too
  much per tick

Because of that, bridge buffers should be treated as a separate project from the
runtime-core data-structure work above. Both are useful, but they remove
different classes of cost.

## Browser Host Architecture Limits

The current project architecture already has the right long-term foundation:

- Rust runtime core
- WASM bridge
- project-owned package format
- narrow host boundary
- browser shell for input, audio, rendering, diagnostics, and package loading

The gap to a stronger browser architecture is mostly in the host layer, not the
parser/package/runtime-core split.

Current host limits:

- runtime ticks are driven from the main browser thread
- the shell uses interval-style scheduling rather than a full fixed-timestep
  accumulator with catch-up and interpolation policy
- frame data crosses the WASM boundary as JSON
- canvas rendering is main-thread 2D canvas
- audio is browser-hosted and subject to browser policy
- no `Worker`, `SharedArrayBuffer`, `OffscreenCanvas`, WebGL/WebGPU renderer, or
  `AudioWorklet` path is currently in use

These limits do not prevent the current MVP from becoming smooth on practical
samples, but they prevent a strict guarantee that browser runtime timing will
match a native EXE in every environment.

## If Architecture Is Not Constrained

The strongest browser-side architecture would be:

- run the WASM runtime core inside a dedicated `Worker`
- use a binary bridge or `SharedArrayBuffer` ring buffers for input, snapshots,
  frame commands, audio events, and diagnostics
- render through `OffscreenCanvas` plus WebGL/WebGPU when draw count or scaling
  makes 2D canvas expensive
- use a fixed-timestep accumulator, bounded catch-up, and optional render
  interpolation
- move audio timing-critical work toward `AudioWorklet` where applicable
- predecode and cache assets before entering gameplay rooms
- enable COOP/COEP headers if `SharedArrayBuffer` is required

This is closer to an emulator-style browser host. It can make the browser path
feel very close to native for supported samples, but it still cannot provide a
perfect native-EXE guarantee because the browser, OS compositor, display refresh
rate, tab lifecycle, and device power policy remain outside the engine's
control.

## Recommended Order

1. Keep release WASM as the default performance path and avoid judging
   smoothness from debug WASM.
2. Implement the runtime-core tick context with reusable scratch buffers.
3. Move collision, place-meeting, and object-target queries onto denser reusable
   spatial/object indexes.
4. Precompute event dispatch and inheritance lookup tables at package/runtime
   load.
5. Replace same-tick update hash maps with sparse overlays.
6. Cache static render-frame data per room if render-frame construction becomes
   visible in measurements.
7. Implement the binary bridge as a separate bridge-contract project.
8. Consider Worker, `SharedArrayBuffer`, `OffscreenCanvas`, WebGL/WebGPU, and
   `AudioWorklet` only when measurements show main-thread host limits, or when
   the project explicitly targets an emulator-style browser host.
