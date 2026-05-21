# WASM Runtime Gap Analysis

This is a living note. Update it whenever parser, runtime-core, runtime-web, or shell behavior changes what the browser can actually play.

## Current Baseline

- The browser shell can load packages, boot the WASM bridge, tick, reset, select rooms, and show telemetry.
- The parser now preserves raw logic in `logic.raw.json` and emits a lightweight lowered contract in `logic.lowered.json`.
- The runtime core has a hardcoded movement/collision/transition baseline, but it does not execute GM8 game logic.

## Necessary Missing

These are the gaps that block normal play. If any of these are absent, the game is not really playable.

### 1. GML Script Execution

The core game logic is still dead.

Current `tick()` behavior is hardcoded movement plus a few runtime diagnostics. Object logic from the room is not executed.

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

Missing pieces include:

- `just_pressed` and `just_released`
- `Keyboard` vs `Key Press`
- mouse click and hover events
- one-shot key press behavior instead of pure level-triggered input

### 6. Lifecycle Event Chain

The object lifecycle is incomplete.

Missing pieces include:

- `instance_create()` -> `Create`
- per-frame `Step` and `Draw`
- collision event dispatch
- `instance_destroy()` -> `Destroy`
- `Clean Up`
- room creation code execution
- instance creation code execution

## Important Missing

These do not always block booting, but they block core IWanna fidelity and make many rooms behave incorrectly.

### 7. Physics Precision

Current movement uses hardcoded constants such as `RUN_SPEED` and `JUMP_SPEED`.

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

## Can Be Deferred

These are real GM8 features, but they do not need to block the first playable runtime slice.

- particles
- timelines
- alarms
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

Current implementation already covers the browser shell, resource loading, bridge boot/tick/reset, and a hardcoded runtime baseline. The gap is the actual GM8 gameplay semantics in the middle.

