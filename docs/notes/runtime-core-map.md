# Runtime Core Map

> Current status note: this document is a readability map for agents working in
> `crates/iwm-runtime-core/`. It describes module ownership and runtime data
> flow; it is not a behavioral spec.

## Purpose

`iwm-runtime-core` owns deterministic package execution after the parser has
emitted a runtime package. It keeps the loaded package, current room state,
runtime instances, globals, input trace, diagnostics, and host-facing render
frames together behind `RuntimeCore`.

Use this map when deciding where a runtime change belongs. Keep parser/package
contract decisions in `iwm-runtime-model` or `iwm-parser`; keep browser shell
concerns in `runtime/` or `iwm-runtime-web`.

## Top-Level Modules

- `lib.rs`: crate exports. Public consumers should normally enter through
  `RuntimeCore` and the runtime snapshot/value types re-exported from `types.rs`.
- `core.rs`: `RuntimeCore` state, package load path, tick loop, snapshots, and
  the main render entrypoint.
- `types.rs`: runtime-owned state and error types used by the core and tests.
- `room_builder.rs`: room boot/build logic, instance construction, sprite
  metrics, collision mask setup, create events, room-start logic, and player
  spawn adjustment.
- `room_transitions.rs`: pending game restart, room reset, room transition, and
  persistent instance carry-over.
- `movement.rs`: built-in player movement, jump/death behavior, room transition
  triggers, and non-player GM motion variables such as speed, direction,
  friction, and gravity.
- `event_dispatch.rs`: event selector lookup, parent fallback, collision spatial
  index construction, and collision target inheritance maps.
- `render.rs`: current-room view sync, `RuntimeRenderFrame` construction, and
  `RuntimeHost::submit_frame` submission.
- `diagnostics.rs`: bounded runtime diagnostic recording, host forwarding, and
  execution trace helpers.
- `debug_input.rs`: input trace/debug snapshot helpers.
- `helpers.rs`: shared low-level runtime predicates and value conversions.
- `logic/`: lowered GML execution for create, room-start, step, collision,
  keyboard, alarm, animation-end, and helper-call behavior.

## Runtime Tick Flow

`RuntimeCore::load()` validates the package, builds id-to-index maps, caches
lowered event/script lookup tables, boots the first room, and stores package
bootstrap globals for later restart/reset paths.

`RuntimeCore::tick(host)` is the main per-frame path:

1. Validate that a room is loaded and start phase timing through the host.
2. Read bound left/right/jump/restart buttons from globals, with default button
   bindings when globals are absent.
3. Increment tick/status and record idle/input diagnostics.
4. Apply any pending room change and sync room views from globals.
5. Execute lowered step events through `logic::execute_lowered_step_events`.
6. If lowered logic requested a scene change, apply it, render, and return.
7. Run built-in player movement unless script movement or jump queries took
   ownership of the player for this tick.
8. Move non-player instances using GM motion variables.
9. Dispatch collision events.
10. Process alarms.
11. Dispatch keyboard held, press, and release events.
12. Convert a restart edge into a pending room reset when applicable.
13. Apply pending room reset, room transition, or game restart.
14. Advance sprite animations and dispatch animation-end events.
15. Render the frame and store tick phase timings.

`RuntimeCore::render(host)` is also callable directly. It settles a newly loaded
room before first render, syncs room views from globals, builds a render frame,
and submits it to the host.

## Lowered Logic Execution

`logic/mod.rs` is the orchestration layer. It owns the public crate-local entry
points used by `RuntimeCore` and delegates expression, statement, assignment,
host-call, and instance-create details to focused submodules.

- `bootstrap.rs`: package/bootstrap globals, create events, room-start events,
  and view globals application.
- `context.rs`: evaluation context, execution scope, room instance overlays,
  binary file state, and per-step result flags.
- `statement.rs`: main lowered statement dispatcher, including scripts, loops,
  with-blocks, and trace collection.
- `eval.rs`: single expression dispatcher used by statement execution.
- `eval_values.rs`: truthiness, binary operations, and value-to-string helpers.
- `eval_variables.rs`: identifier/member/index lookup and assignable key
  resolution.
- `eval_functions.rs`: GM helper calls such as random, choose, keyboard,
  place/collision, file, instance, and distance helpers.
- `assignment.rs`: local, instance, global, room id, and view-global assignment.
- `control_flow.rs`: scene-change interruption checks, with-target iteration,
  and overlay merge/sync helpers.
- `instances.rs`: instance member assignment, pending creates, and
  `instance_create` request construction.
- `calls.rs`: host-backed calls, including audio and binary file helpers.
- `diagnostics.rs`: unsupported statement/function/expression diagnostic
  formatting for lowered logic.

`execute_lowered_step_events()` snapshots dispatch targets from the current
room, builds a `RuntimeEvalContext` and `RuntimeStatementEnvironment` for each
statement, applies `apply_runtime_statement()`, commits pending overlays and
creates, and stops early when a scene change interrupts the tick.

## Diagnostics And Host Boundary

The runtime host boundary is intentionally narrow:

- `core.rs` reads active/button state, asks the host for diagnostic timing, and
  records runtime diagnostics.
- `render.rs` builds `RuntimeRenderFrame` values and submits them through
  `RuntimeHost::submit_frame`.
- `logic/calls.rs` invokes host audio/file behavior for supported GM helper
  calls.
- `logic/context.rs` stores binary file state used by file helper calls.
- `diagnostics.rs` mirrors bounded diagnostics into local runtime state and the
  host recorder.

Diagnostics should stay structured and actionable. For lowered-logic blockers,
include the unsupported statement/function/expression, first-trigger room or
event context when available, and the runtime instance or object involved.

## When Changing Runtime-Core

- Keep package shape changes synchronized with `docs/notes/package-format-v1-runtime.md`.
- Keep playable-runtime blocker changes synchronized with
  `docs/notes/runtime-wasm-gap-analysis.md`.
- Add or adjust tests at the closest layer first: unit for pure helpers,
  scenario tests for visible runtime behavior, and contract tests for boundary
  shape.
- Prefer extending the focused `logic/` module that owns the behavior instead of
  growing `logic/mod.rs` or `statement.rs`.
- Keep browser-specific assumptions outside this crate unless they are part of
  the explicit host boundary.
