# WASM Runtime Gap Analysis

This is a living note. Update it whenever parser, runtime-core, runtime-web, or shell behavior changes what the browser can actually play.

## Current Baseline

- The browser shell can load packages, boot the WASM bridge, tick, reset, select rooms, and show telemetry.
- The parser now preserves raw logic in `logic.raw.json` and emits a lightweight lowered contract in `logic.lowered.json`.
- The runtime core now consumes a small create-time slice and a narrow `step` slice of `logic.lowered.json` for bootstrapping assignments plus direct `room_goto` / `game_restart` / assignment semantics, but it still does not execute general GM8 gameplay logic.

## Necessary Missing

These are the gaps that block normal play. If any of these are absent, the game is not really playable.

### 1. GML Script Execution

The core game logic is still mostly dead.

Current `tick()` behavior is hardcoded movement plus a few runtime diagnostics, with only a very small lowered-logic slice dispatched for `step` events. Most object logic from the room is still not executed.

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

- `just_pressed` and `just_released`
- `Keyboard` vs `Key Press`
- mouse click and hover events
- one-shot key press behavior instead of pure level-triggered input

### 6. Lifecycle Event Chain

The object lifecycle is incomplete.

Current rendering exists at the room/frame level, but GML event-driven lifecycle execution does not.

Missing pieces include:

- `instance_create()` -> `Create`
- per-frame `Step`
- `Draw event` logic execution
- collision event dispatch
- `instance_destroy()` -> `Destroy`
- `Clean Up`
- room creation code execution
- instance creation code execution

## Important Missing

These do not always block booting, but they block core IWanna fidelity and make many rooms behave incorrectly.

### 7. Physics Precision

Current movement uses hardcoded constants such as `RUN_SPEED` and `JUMP_SPEED`.

The runtime already has a hardcoded player movement baseline and per-instance `hspeed` / `vspeed` fields, but not a general GM8-style object-driven physics model.

Missing pieces include:

- per-object `friction`
- per-object `gravity`
- per-object `hspeed` / `vspeed`
- frame-accumulated gravity rather than a single hardcoded motion model

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

The parser already preserves `parent_index` in object definitions, but runtime inheritance semantics do not use that data yet.

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

Alarm logic is not yet implemented.

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

- execute GML
- store variables
- dispatch Create / Step / Collision / Destroy
- react to keyboard edges
- animate sprites
- play audio
- support room transitions and deaths as real game events

Current implementation already has a hardcoded baseline for player movement, AABB collision, reset, room switching, frame submission, and browser-hosted telemetry. The missing middle layer is the actual GM8 gameplay semantics: GML execution, variables, lifecycle dispatch, animation, and audio.
