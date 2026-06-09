# WASM Runtime Gap Analysis

> **Current status note:** Keep this document synchronized with actual runtime-core, runtime-web, and shell behavior.
>
> If code changes reduce or introduce playable-runtime blockers, update this note in the same change.

This is a living note. Update it whenever parser, runtime-core, runtime-web, or shell behavior changes what the browser can actually play.

## Current Baseline

- The browser shell can load packages, boot the WASM bridge, auto-run it at a shell-driven 60 Hz tick, pause/resume that loop, reset, select rooms, and show telemetry.
- The parser now preserves raw logic in `logic.raw.json` and emits a structured lowered contract in `logic.lowered.json` for the current IWanna-critical subset.
- The parser now also emits sprite collision bounds in `resources/index.json` as `bbox_left`, `bbox_right`, `bbox_top`, and `bbox_bottom`, plus optional `collision_masks` sourced from OpenGMK collision metadata.
- The lowered parser contract now covers common comment stripping, `var` declarations, assignments, returns, calls, member/index access, unary expressions, and common control-flow heads on the current critical path.
- The runtime core now consumes a small create-time slice and a narrow `step` slice of `logic.lowered.json` for bootstrapping assignments plus direct `room_goto` / `game_restart` / assignment semantics, and it now also dispatches alarm, held-key, key-press, and key-release slices with parent fallback lookup for event dispatch.
- The runtime core now uses a variable-height jump state machine on the IWanna-critical path, including held jump differentiation, release-cut tracking, ceiling-hit phase clearing, and landing reset state clearing.
- The runtime core now also evaluates `keyboard_check`, `keyboard_check_direct`, `keyboard_check_pressed`, `keyboard_check_released`, `place_meeting`, `place_free`, `&&`, `||`, and single-`=` GM comparisons on the current lowered path, and player motion now preserves floating-point `x/y/hspeed/vspeed` plus subpixel axis deltas instead of rounding assignments or movement back to integers.
- The runtime core now executes `var` declarations through a per-lowered-entry local scope for runtime event execution. Local variables can feed later expressions in the same block without leaking into instance variables, script calls receive their own local scope, and unresolved identifiers no longer silently become text values during ordinary variable reads.
- The runtime core now executes lowered `with` blocks on the current runtime event path for object-name targets plus `self`, `other`, and `all`. Step-event execution now evaluates each lowered statement against a merged live room-state view, so same-event object-member reads, `instance_exists`, and nested `with(other)` observe pending `with` writes and destroys instead of the stale pre-event room snapshot.
- The runtime core now handles `instance_destroy()` on the lowered runtime event path. The call dispatches the target instance's `Destroy` event before marking that instance non-live, and `with (...) { instance_destroy(); }` applies to the current `with` target rather than the caller.
- The runtime core now also handles `instance_create()` on the lowered runtime event path. Runtime-created instances can duplicate an existing object type, run their own `Create` event immediately, and collision events can now call `instance_destroy()` to run the owner's `Destroy` event before removing it from live runtime participation.
- The browser/input path no longer needs jump to be hardcoded to Space for runtime-core fallback movement; runtime fallback input now prefers package-initialized bindings such as `global.leftbutton`, `global.rightbutton`, and `global.jumpbutton`, while the browser-facing host can also forward raw virtual-key hold/press/release state alongside the shell's semantic controls.
- The browser shell no longer maps `W` / `ArrowUp` / `Space` into a shell-side semantic jump boolean, and the web-runtime host no longer aliases semantic jump edges onto `VK_SPACE`; jump intent now reaches runtime primarily through raw forwarded GM key codes so package-owned bindings such as `global.jumpbutton = vk_shift` can control which physical key actually jumps.
- The runtime core now also re-resolves the package-bound jump key after lowered `step` logic runs, so a same-tick script update such as `global.jumpbutton = vk_shift` can affect builtin fallback movement on that same frame instead of one frame late.
- When lowered player step logic already queries `keyboard_check*` against `global.jumpbutton`, runtime-core now treats jump as script-owned for that frame and suppresses builtin jump injection while still preserving fallback movement/gravity progression around the scripted vertical path.
- The browser-facing host path now treats one-shot controls such as jump/restart as host-boundary input edges and clears edge bits after each tick; the shell now drives those per-tick inputs through a 60 Hz auto-run loop instead of only a manual single-step button. The next runtime blocker is broader OpenGMK semantic coverage, not expanding shell-side gameplay rules.
- The browser shell / wasm-session path now also accumulates raw key press/release edges until the next runtime tick instead of deriving edges only from the latest held-key snapshot, so very short taps that start and end within one shell interval are no longer silently lost before reaching the runtime host.
- The current lowered runtime slice now also resolves `file_exists()` against a small sampled host-file set (`temp`, `DeathTime`, `save1`-`save3`), which is enough to advance more of the `rInit` / save bootstrap path without yet claiming general GM8 file API coverage.
- Parser-built packages now preserve GM room order as `manifest.room_order`; runtime boot and `room_goto_next()` use that order, so title/menu/select rooms can follow the original room chain instead of the previous JSON-array ordering.
- Runtime room construction no longer injects a fallback player into rooms without explicit spawn state. Player creation must come from a room instance or currently supported spawn logic, and runtime-core now dispatches `other:room-start` blocks during room build so `playerStart`-style spawn objects can create the player through original room-start logic.
- The runtime core now also hydrates missing package bootstrap globals before shell-driven manual `select_room` / `reload_room` jumps, using parser-lowered room-instance create blocks that assign `global.*`. This specifically fixes sample-package hand testing where direct entry into a playable room previously skipped required globals such as `global.grav`, making second jumps fail even though `Shift` press/release edges and `playerJump()` dispatch were already correct.
- Package validation now accepts hidden room background layers that reference non-exported resources, matching the current renderer contract, while visible room backgrounds and tile backgrounds remain hard references.
- Runtime snapshots and the browser shell now also expose jump-trace telemetry for the current player path: grounded state plus active / hold / cut jump-phase flags. This is a debugging and validation surface, not proof that the underlying jump semantics already match the gold sample.
- The browser shell now also exposes per-frame runtime timing telemetry for the WASM path, including separate input, tick, snapshot, frame, canvas render, total frame, draw command count, and skipped auto-tick interval values. Runtime snapshots also carry the previous runtime-core tick phase timings for input diagnostics, lowered step events, view sync, player movement, collision events, alarms, keyboard events, and render submission. This makes large-room slowdowns visible when the shell cannot finish a tick/render cycle inside the 60 Hz interval, and separates runtime-core work from frame JSON and browser drawing.
- Runtime-core player fallback movement now filters solid and hazard collision candidates to instances near the player's current motion envelope before running bbox and sprite-mask collision checks, and it no longer clones the full room instance list before filtering those candidates.
- Runtime-core lowered step dispatch now shares host input/file sampling across all step owners in a tick and evaluates against the original room instance slice without cloning the full room snapshot. Large rooms can still be expensive because broader event dispatch and render-frame command generation continue to scan full room instance/tile lists.
- Runtime-core collision event dispatch now indexes live instances by `object_id` before checking collision targets, so large rooms with many collision-event owners targeting the player do not scan every room instance for each owner.
- Runtime-core lowered execution now emits block-level execution trace diagnostics and unsupported lowered-runtime diagnostics with the current room, tick, block id, object name, event tag, and runtime instance id. Unsupported statements are reported by lowered statement kind, and unsupported function calls are reported both when they appear as statements and when they appear inside evaluated expressions. The developer CLI can aggregate these with `cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --select-room <room_id> --ticks <n>`.
- Runtime-core expression evaluation now covers the Dife-critical helpers `abs()`, `string()`, and object-name `distance_to_object()`. A local Dife diagnostics run against `sampleroom01` (`room_id = 143`) with `cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --select-room 143 --ticks 60 --press-keys 16` now ranks the next blockers as `runtime-unsupported-function:collision_line` and `runtime-unsupported-statement:for`.

Practical parser note:

- broad `runtime-missing-source-lowering:*` warnings from `analysis.json` still need gold-sample evidence before they should be treated as real blockers
- a `source-only` `scripts.ir.json` block can already have a usable structured `logic.lowered.json` entry, so warning interpretation should follow the lowered contract, not only the older IR support label

Practical contract note:

- runtime progress still depends on cross-file package integrity, so parser/runtime/web work should treat identity/reference validation as a first-class prerequisite rather than as browser-only debugging
- `crates/iwm-runtime-model/` now provides `validate_runtime_package()` plus a checked-in sparse synthetic fixture, and `iwm-cli validate-package` exposes the same structural checks for local generated packages
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
- broader `with` parity still depends on the remaining expression, lifecycle, and instance-id semantics even though the current lowered runtime slice now switches object/self/other/all execution context

### 2. Audio

Background music and sound effects are missing.

GM8 sound functions such as `sound_play()` and `sound_loop()` still need a Web Audio implementation.

### 3. Variable System

GML is dynamic, so assignments must work against a variable store, not only hardcoded fields.

Current runtime instances already carry a small hardcoded state set such as `x`, `y`, `hspeed`, and `vspeed`, but that is not a general GML variable system.

Missing pieces include:

- broader `global.var` coverage beyond the current lowered assignment/read slice
- broader instance-local variable behavior beyond the current runtime `vars` map and hardcoded movement fields
- full `var tmp` behavior beyond the current per-entry runtime local scope
- array access such as `array[0] = value`
- property access on objects and instances

Current status:

- lowered runtime execution now has a scoped local store for `var` declarations inside event/script execution
- locals can be assigned and read by later expressions in the same lowered entry
- script calls use an isolated local scope instead of leaking `var` state back into the caller
- ordinary unresolved identifiers no longer fall back to text values; symbol-name function arguments such as object names are still handled by the specific runtime helper that consumes them

This gap is partly runtime-side and partly parser-side. The parser now carries structured member/index/binary nodes on the critical path, and runtime execution consumes a small scoped-variable slice, but it still does not consume the full dynamic variable model.

### 4. Sprite Animation

Sprites are still effectively static.

Missing pieces include:

- `image_index` progression
- `image_speed`
- frame looping / wraparound
- per-frame animation advancement

### 5. Keyboard And Mouse Edge Events

GM8 distinguishes held state from edge-triggered state.

Current input handling now has both semantic shell controls and raw-key forwarding through the browser bridge, and runtime-core keyboard queries can resolve against package-initialized GM key bindings. The full GM8 keyboard and mouse event model is still missing.

Missing pieces include:

- mouse click and hover events
- one-shot key press behavior for the broader key map instead of only the browser shell's current subset

Current status:

- held, press, and release dispatch now exists for the current runtime core keyboard slice
- runtime-core query functions now resolve against host key state instead of only shell-hardcoded jump booleans
- the remaining gap is broader GM8 input coverage beyond the shell/runtime buttons currently wired in the bridge, plus mouse semantics

### 6. Lifecycle Event Chain

The object lifecycle is incomplete.

Current rendering exists at the room/frame level, but GML event-driven lifecycle execution does not.

Missing pieces include:

- per-frame `Step`
- `Draw event` logic execution
- collision event dispatch beyond selector and lookup coverage
- `Clean Up`
- full room creation code execution beyond the currently lowered bootstrap subset
- full instance creation code execution beyond the currently lowered bootstrap subset
- full Other-event coverage beyond the current `other:room-start` spawn path

Current status:

- `instance_destroy()` now marks the current lowered runtime execution instance dead after running that instance's `Destroy` event on the shared event path
- `instance_create()` now works on the shared lowered runtime event path and runs the new instance's `Create` event without suppressing duplicate object instances
- collision events can now participate in this lifecycle chain by destroying their owner through lowered `instance_destroy()`
- full death / respawn presentation still depends on broader collision dispatch, object creation semantics, sprite animation, draw handling, and sample-specific R-key/restart lifecycle parity

For current planning purposes, `keyboard`, `collision`, `alarm`, and room-start handling should be treated as part of the first IWanna-critical lifecycle slice rather than as optional polish. Keyboard, alarm, and room-start dispatch now exist in the current runtime slice; collision lookup is wired for selection and test coverage, but full runtime collision dispatch remains deferred.

## Important Missing

These do not always block booting, but they block core IWanna fidelity and make many rooms behave incorrectly.

### 7. Physics Precision

Current movement still uses some hardcoded defaults such as `RUN_SPEED` and fallback jump values, but jump is no longer a fixed-height placeholder, and motion assignments / axis deltas no longer lose GM8 fractional values on write or movement.

The runtime already has a hardcoded player movement baseline and per-instance `hspeed` / `vspeed` fields, plus explicit jump-phase state for hold, cut, and landing-reset behavior, but not a general GM8-style object-driven physics model.

Missing pieces include:

- per-object `friction`
- per-object `gravity`
- per-object `hspeed` / `vspeed`
- frame-accumulated gravity rather than a single hardcoded motion model
- numeric jump calibration against the `IWBT_Dife` gold sample instead of only generic hold/cut semantics

Practical current note:

- browser smoke after these changes shows that jump-path blockers have moved from "input and fractional values are dead" to broader lifecycle/runtime coverage; the player can still end up in obviously wrong long-run room states because `rInit`/room-start/world initialization semantics remain incomplete
- do not treat the new floating-point/input-query support as proof that native IWanna jump feel is solved end to end yet
- jump height calibration should now account for preserved subpixel vertical motion; if a held or double jump still falls short, the next likely causes are missing lowered semantics in the sample player Step path or unsupported GM helper calls rather than implicit integer movement truncation

### 8. Views And Cameras

GM8 viewports and camera following are not fully implemented.

Current status:

- parser-built packages preserve view source rectangles, view ports, follow
  target, and follow border/speed metadata
- runtime-core renders the first visible view as the frame surface instead of
  sizing browser frames to the full room
- the current lowered runtime slice supports the gold sample's fixed-screen
  camera pattern through `view_xview` / `view_yview` assignments
- player fallback movement no longer checks every solid/hazard in a large room
  for each collision probe and no longer clones the full room instance list
  before filtering nearby candidates
- lowered step dispatch no longer clones the full room instance snapshot or
  repeats known-file host sampling for each step owner
- frame generation and general event dispatch are not yet spatially indexed
- collision event dispatch now narrows candidates by target `object_id` before
  running collision checks

Still missing:

- full multi-view rendering
- `view_angle` rendering
- full GM8 follow-target camera updates across object ids and instance ids
- outside/intersect-view event parity
- spatial indexing or cached visible slices for room tiles, visible instances,
  and remaining full-room scans

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
- advanced collision transforms such as scaled / rotated precise masks
- D&D action execution for non-GML-heavy games
- external DLL semantics
- advanced drawing APIs
- full menu/save-select systems beyond basic room-order and room-start transitions

## Minimum Playable Runtime

For IWanna-style games, the runtime is only meaningfully playable when it can do all of the following:

- execute GML for the current lowered subset
- store variables
- dispatch Create / Step / Collision / Destroy and alarm / key-edge slices
- react to keyboard edges
- animate sprites
- play audio
- support room transitions and deaths as real game events

Current implementation already has a hardcoded baseline for player movement, bbox broad-phase plus sprite-mask pixel collision, reset, room switching, frame submission, and browser-hosted telemetry. The missing middle layer is the actual GM8 gameplay semantics: GML execution, variables, lifecycle dispatch, animation, and audio.
The browser can now start from the original `rInit` order and advance toward title/menu/select rooms through lowered room logic, but full playability still depends on Draw-event text/sprite behavior, file/save APIs such as `file_bin_*`, and broader menu object logic.

Current jump-validation note:

- jump-state trace coverage now exists in crate-local tests for tap vs hold, release cut, ceiling collision phase clearing, and landing reset
- the runtime snapshot / wasm bridge / shell telemetry path now surfaces grounded and jump-phase state live during browser execution, so hand-feel debugging no longer depends only on Rust test fixtures
- same-tick binding changes and within-tick raw key tap edges now have dedicated regression coverage across `runtime-core`, `runtime-web`, and the TS wasm-session bridge
- runtime-core now has regression coverage for preserving fractional vertical jump motion with `jump=8.5` and `gravity=0.4`, matching the gold sample's first-jump variables
- sample-accurate numeric alignment still requires local `IWBT_Dife` package validation rather than only repository fixtures

Current resource-contract note:

- sprite collision bounds are exported as an aggregated rectangle per sprite, and `collision_masks` preserve the gm8exe bool maps for runtime pixel collision. The current runtime consumes the first available mask for each sprite; full `image_index` / animated per-frame mask selection is still deferred.

## Immediate Priority Order

The current route sets the next implementation order as:

1. use runtime unsupported diagnostics and block-level trace output to rank the active Dife path before adding more GM helpers
2. keep the shared lowered parser contract stable except where gold-sample evidence requires targeted expansion
3. headless OpenGMK-derived runtime extraction behind narrow host traits
4. browser WASM host integration for that runtime core
5. audio, animation, and broader lifecycle coverage after the runtime can execute trustworthy semantics
