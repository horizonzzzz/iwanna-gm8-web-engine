# WASM Runtime Gap Analysis

> **Current status note:** Keep this document synchronized with actual runtime-core, runtime-web, and shell behavior.
>
> If code changes reduce or introduce playable-runtime blockers, update this note in the same change.

This is a living note. Update it whenever parser, runtime-core, runtime-web, or shell behavior changes what the browser can actually play.

## Current Baseline

- The browser shell can load packages, boot the WASM bridge, tick, reset, select rooms, and show telemetry.
- The parser now preserves raw logic in `logic.raw.json` and emits a structured lowered contract in `logic.lowered.json` for the current IWanna-critical subset.
- The parser now also emits sprite collision bounds in `resources/index.json` as `bbox_left`, `bbox_right`, `bbox_top`, and `bbox_bottom`, sourced from OpenGMK collision metadata.
- The lowered parser contract now covers common comment stripping, `var` declarations, assignments, returns, calls, member/index access, unary expressions, and common control-flow heads on the current critical path.
- The runtime core now consumes a small create-time slice and a narrow `step` slice of `logic.lowered.json` for bootstrapping assignments plus direct `room_goto` / `game_restart` / assignment semantics, and it now also dispatches alarm, held-key, key-press, and key-release slices with parent fallback lookup for event dispatch.
- The runtime core now uses a variable-height jump state machine on the IWanna-critical path, including held jump differentiation, release-cut tracking, ceiling-hit phase clearing, and landing reset state clearing.
- The browser-facing host path now treats one-shot controls such as jump/restart as host-boundary input edges and clears edge bits after each tick; the next runtime blocker is broader OpenGMK semantic coverage, not expanding shell-side gameplay rules.

Practical parser note:

- broad `runtime-missing-source-lowering:*` warnings from `analysis.json` still need gold-sample evidence before they should be treated as real blockers
- a `source-only` `scripts.ir.json` block can already have a usable structured `logic.lowered.json` entry, so warning interpretation should follow the lowered contract, not only the older IR support label

Practical contract note:

- runtime progress still depends on cross-file package integrity, so parser/runtime/web work should treat identity/reference validation as a first-class prerequisite rather than as browser-only debugging
- recent regressions showed that unresolved package references such as sparse `object_id` handling can look like rendering bugs even when the real fault is contract consumption drift
- event dispatch now also depends on preserving `parent_index` as a runtime lookup path rather than assuming dense object arrays imply direct ownership

## Route Decision

The repository now treats the runtime and parser problems as two coupled tracks, with one mainline decision:

- runtime mainline: move toward an OpenGMK-derived execution core through narrow project-owned host boundaries
- parser mainline: replace shallow token splitting with a real parser-owned structure for the IWanna-critical subset

This note therefore tracks both kinds of blocker:

- runtime-semantic blockers that require deeper runner behavior
- parser-contract blockers that prevent runtime code from receiving executable structure in the first place

## Necessary Missing

These are the gaps that block normal play. If any of these are absent, the game is not really playable.

### 1. GML Script Execution

The core game logic is still mostly dead.

Current `tick()` behavior is hardcoded movement plus a few runtime diagnostics, with only a very small lowered-logic slice dispatched for `step` events. Most object logic from the room is still not executed.

The blocker is no longer interpreted as "add more heuristics to the TS runtime". The real blocker is that the runtime path still lacks a trustworthy executable contract for common GML calls, expressions, event dispatch, and variable lookup.

Impact:

- enemies do not run their `Step` logic
- bullets do not spawn
- traps do not trigger
- doors do not open
- score / state logic does not run
- `with`-targeted logic is effectively ignored

### 2. Audio

Background music and sound effects are missing.

GM8 sound functions such as `sound_play()` and `sound_loop()` still need a Web Audio implementation.

### 3. Variable System

GML is dynamic, so assignments must work against a variable store, not only hardcoded fields.

Current runtime instances already carry a small hardcoded state set such as `x`, `y`, `hspeed`, and `vspeed`, but that is not a general GML variable system.

Missing pieces include:

- `global.var`
- instance-local variables
- `var tmp` locals
- array access such as `array[0] = value`
- property access on objects and instances

This gap is partly runtime-side and partly parser-side. The parser now carries structured member/index/binary nodes on the critical path, but runtime execution still does not consume the full dynamic variable model.

### 4. Sprite Animation

Sprites are still effectively static.

Missing pieces include:

- `image_index` progression
- `image_speed`
- frame looping / wraparound
- per-frame animation advancement

### 5. Keyboard And Mouse Edge Events

GM8 distinguishes held state from edge-triggered state.

Current input handling already has a partial edge-triggered basis for `jump` and `restart` through the browser bridge and host button snapshots, but the full GM8 keyboard and mouse event model is still missing.

Missing pieces include:

- mouse click and hover events
- one-shot key press behavior for the broader key map instead of only the browser shell's current subset

Current status:

- held, press, and release dispatch now exists for the current runtime core keyboard slice
- the remaining gap is broader GM8 input coverage beyond the shell/runtime buttons currently wired in the bridge

### 6. Lifecycle Event Chain

The object lifecycle is incomplete.

Current rendering exists at the room/frame level, but GML event-driven lifecycle execution does not.

Missing pieces include:

- `instance_create()` -> `Create`
- per-frame `Step`
- `Draw event` logic execution
- collision event dispatch beyond selector and lookup coverage
- `instance_destroy()` -> `Destroy`
- `Clean Up`
- room creation code execution
- instance creation code execution

For current planning purposes, `keyboard`, `collision`, and `alarm` handling should be treated as part of the first IWanna-critical lifecycle slice rather than as optional polish. Keyboard and alarm dispatch now exist in the current runtime slice; collision lookup is wired for selection and test coverage, but full runtime collision dispatch remains deferred.

## Important Missing

These do not always block booting, but they block core IWanna fidelity and make many rooms behave incorrectly.

### 7. Physics Precision

Current movement still uses some hardcoded defaults such as `RUN_SPEED` and fallback jump values, but jump is no longer a fixed-height placeholder.

The runtime already has a hardcoded player movement baseline and per-instance `hspeed` / `vspeed` fields, plus explicit jump-phase state for hold, cut, and landing-reset behavior, but not a general GM8-style object-driven physics model.

Missing pieces include:

- per-object `friction`
- per-object `gravity`
- per-object `hspeed` / `vspeed`
- frame-accumulated gravity rather than a single hardcoded motion model
- numeric jump calibration against the `IWBT_Dife` gold sample instead of only generic hold/cut semantics

### 8. Views And Cameras

GM8 viewports and camera following are not fully implemented.

Missing pieces include:

- view cropping
- following different objects
- viewport / port sizing
- multi-view behavior

### 9. Room Persistence

Persistent instances are not preserved across room changes.

Missing pieces include:

- `persistent = true` objects surviving room transitions
- reusing existing persistent instances instead of rebuilding the room from scratch

### 10. Object Inheritance

Parent/child object inheritance is not fully modeled.

The parser already preserves `parent_index` in object definitions, and runtime event lookup now follows the parent chain for matching event blocks. Full inheritance semantics still do not use that data for variable defaults or broader object behavior.

Missing pieces include:

- `parent_index` inheritance chain
- inherited event fallback
- inherited variable defaults
- collision / categorization behavior that respects the chain

### 11. Alpha And Blend

Rendering still ignores transparency and blend settings.

Missing pieces include:

- `image_alpha`
- `image_blend`
- instance / sprite-level alpha handling

### 12. Alarm

Alarm logic now exists in the current runtime slice for countdown-triggered event dispatch, but broader semantic parity is still incomplete.

This is not always a first-room blocker, but many fangame traps, delayed spikes, and boss patterns depend on it. Treat it as an important missing feature and promote it to necessary if the active gold sample depends on alarms on the first playable path.

## Can Be Deferred

These are real GM8 features, but they do not need to block the first playable runtime slice.

- particles
- timelines
- surface rendering
- save / load
- pixel-perfect collision masks
- D&D action execution for non-GML-heavy games
- external DLL semantics
- advanced drawing APIs
- menu systems

## Minimum Playable Runtime

For IWanna-style games, the runtime is only meaningfully playable when it can do all of the following:

- execute GML for the current lowered subset
- store variables
- dispatch Create / Step / Collision / Destroy and alarm / key-edge slices
- react to keyboard edges
- animate sprites
- play audio
- support room transitions and deaths as real game events

Current implementation already has a hardcoded baseline for player movement, AABB collision, reset, room switching, frame submission, and browser-hosted telemetry. The missing middle layer is the actual GM8 gameplay semantics: GML execution, variables, lifecycle dispatch, animation, and audio.

Current jump-validation note:

- jump-state trace coverage now exists in crate-local tests for tap vs hold, release cut, ceiling collision phase clearing, and landing reset
- sample-accurate numeric alignment still requires local `IWBT_Dife` package validation rather than only repository fixtures

Current resource-contract note:

- sprite collision bounds are currently exported as one aggregated rectangle per sprite, derived from the gm8exe collision maps; the package does not yet expose per-frame collision rectangles as separate runtime resources

## Immediate Priority Order

The current route sets the next implementation order as:

1. keep the shared lowered parser contract stable except where gold-sample evidence requires targeted expansion
2. headless OpenGMK-derived runtime extraction behind narrow host traits
3. browser WASM host integration for that runtime core
4. audio, animation, and broader lifecycle coverage after the runtime can execute trustworthy semantics
