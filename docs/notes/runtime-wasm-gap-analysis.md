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
- The runtime core now also evaluates `keyboard_check`, `keyboard_check_direct`, `keyboard_check_pressed`, `keyboard_check_released`, `keyboard_get_numlock`, `place_meeting`, `place_free`, `instance_place`, `&&`, `||`, and single-`=` / `==` / `!=` GM comparisons on the current lowered path, executes `keyboard_set_numlock` against a spoofed host input state, and player motion now preserves floating-point `x/y/hspeed/vspeed` plus subpixel axis deltas instead of rounding assignments or movement back to integers.
- The runtime core now executes `var` declarations through a per-lowered-entry local scope for runtime event execution. Local variables can feed later expressions in the same block without leaking into instance variables, script calls receive their own local scope, and unresolved identifiers no longer silently become text values during ordinary variable reads. Object-name identifiers on the lowered runtime path can now also resolve to runtime object ids when a helper or comparison expects the GM object constant.
- The runtime core now executes lowered `with` blocks on the current runtime event path for object-name targets plus `self`, `other`, and `all`. Step-event execution now evaluates each lowered statement against a merged live room-state view, so same-event object-member reads, `instance_exists`, and nested `with(other)` observe pending `with` writes and destroys instead of the stale pre-event room snapshot.
- The runtime core now handles `instance_destroy()` on the lowered runtime event path. The call dispatches the target instance's `Destroy` event before marking that instance non-live, and `with (...) { instance_destroy(); }` applies to the current `with` target rather than the caller.
- The runtime core now also handles `instance_create()` on the lowered runtime event path, including expression-form calls that return a created-instance reference for follow-up member writes such as `b.direction = i`. Runtime-created instances can duplicate an existing object type, run their own `Create` event, accept post-create member assignments, and collision events can now call `instance_destroy()` to run the owner's `Destroy` event before removing it from live runtime participation. Lowered `repeat` bodies now execute with an iteration guard, which covers Dife-style particle bursts inside emitter Step events.
- Non-player runtime instances now also advance their own `x/y` by lowered `hspeed/vspeed` after step execution, which is enough for the current Dife bullet path to move, probe `place_meeting(x + hspeed, y, block)`, and destroy itself from its own lowered `step` logic instead of only expiring through `alarm[0]`.
- The browser/input path no longer needs jump to be hardcoded to Space for runtime-core fallback movement; runtime fallback input now prefers package-initialized bindings such as `global.leftbutton`, `global.rightbutton`, and `global.jumpbutton`, while the browser-facing host can also forward raw virtual-key hold/press/release state alongside the shell's semantic controls.
- The browser shell no longer maps `W` / `ArrowUp` / `Space` into a shell-side semantic jump boolean, and the web-runtime host no longer aliases semantic jump edges onto `VK_SPACE`; jump intent now reaches runtime primarily through raw forwarded GM key codes so package-owned bindings such as `global.jumpbutton = vk_shift` can control which physical key actually jumps.
- The runtime core now also re-resolves the package-bound jump key after lowered `step` logic runs, so a same-tick script update such as `global.jumpbutton = vk_shift` can affect builtin fallback movement on that same frame instead of one frame late.
- When lowered player step logic already queries `keyboard_check*` against `global.jumpbutton`, runtime-core now treats jump as script-owned for that frame and suppresses builtin jump injection while still preserving fallback movement/gravity progression around the scripted vertical path.
- The browser-facing host path now treats one-shot controls such as jump/restart as host-boundary input edges and clears edge bits after each tick; the shell now drives those per-tick inputs through a 60 Hz auto-run loop instead of only a manual single-step button. The next runtime blocker is broader OpenGMK semantic coverage, not expanding shell-side gameplay rules.
- Runtime-core restart input now resolves runtime/package globals such as `global.restartbutton` and `global.resetbutton` before falling back to host key `R` (`82`). Treat `R` as the current fallback reset key, not as a hardcoded IWanna rule. Runtime-core no longer automatically restarts the room on direct hazard death; reset remains an explicit input edge.
- The browser shell / wasm-session path now also accumulates raw key press/release edges until the next runtime tick instead of deriving edges only from the latest held-key snapshot, so very short taps that start and end within one shell interval are no longer silently lost before reaching the runtime host.
- The current lowered runtime slice now also resolves `file_exists()` against a small sampled host-file set (`temp`, `DeathTime`, `save1`-`save3`) and supports a minimal binary-file slice for `file_bin_open()`, `file_bin_read_byte()`, `file_bin_write_byte()`, and `file_bin_close()`. This is enough for the current Dife `DeathTime` read/write path and related save bootstrap logic, but it is not a claim of general GM8 file API coverage.
- Parser-built packages now preserve GM room order as `manifest.room_order`; runtime boot and `room_goto_next()` use that order, so title/menu/select rooms can follow the original room chain instead of the previous JSON-array ordering.
- Runtime room construction no longer injects a fallback player into rooms without explicit spawn state. Player creation must come from a room instance or currently supported spawn logic, and runtime-core now dispatches `other:room-start` blocks during room build so `playerStart`-style spawn objects can create the player through original room-start logic.
- The runtime core now also hydrates missing package bootstrap globals before shell-driven manual `select_room` / `reload_room` jumps, using parser-lowered room-instance create blocks that assign `global.*`. This specifically fixes sample-package hand testing where direct entry into a playable room previously skipped required globals such as `global.grav`, making second jumps fail even though `Shift` press/release edges and `playerJump()` dispatch were already correct.
- Package validation now accepts hidden room background layers that reference non-exported resources, matching the current renderer contract, while visible room backgrounds and tile backgrounds remain hard references.
- Runtime snapshots and the browser shell now also expose jump-trace telemetry for the current player path: grounded state plus active / hold / cut jump-phase flags. This is a debugging and validation surface, not proof that the underlying jump semantics already match the gold sample.
- The browser shell now also exposes per-frame runtime timing telemetry for the WASM path, including separate input, tick, snapshot, frame, canvas render, total frame, draw command count, and skipped auto-tick interval values. Runtime snapshots also carry the previous runtime-core tick phase timings for input diagnostics, lowered step events, view sync, player movement, collision events, alarms, keyboard events, and render submission. This makes large-room slowdowns visible when the shell cannot finish a tick/render cycle inside the 60 Hz interval, and separates runtime-core work from frame JSON and browser drawing.
- The browser shell now also exposes a copy-first plain-text runtime report covering runtime status, room, tick, player, input, performance, tick phases, recent runtime events, and diagnostics. This is now the primary shell debugging surface for one-shot sharing and comparison; package inspectors remain available as secondary read-only tabs.
- Runtime-core player fallback movement now filters solid and hazard collision candidates to instances near the player's current motion envelope before running bbox and sprite-mask collision checks, and it no longer clones the full room instance list before filtering those candidates.
- Runtime-core lowered step dispatch now shares host input/file sampling across all step owners in a tick and evaluates against the original room instance slice without cloning the full room snapshot. Large rooms can still be expensive because broader event dispatch and render-frame command generation continue to scan full room instance/tile lists.
- Runtime-core lowered step expression evaluation now also keeps a per-tick `object_id -> instance indices` lookup and a borrowed same-tick update overlay, so object-target queries such as `instance_number(player)`, `distance_to_object(player)`, `place_meeting(..., player)`, and `with(player)` do not repeatedly scan and clone the full room instance list for every step owner. This specifically reduces large-room stalls in rooms with many per-instance step blocks such as the local Dife `rMegaman01` smoke case, where repeated player-target queries were dominating the `step` phase.
- Runtime-core collision event dispatch now indexes live instances by `object_id` before checking collision targets, expands collision targets through child objects that inherit from the declared target, and still dispatches the original parent-target event block. This lets Dife player collision events against parent `playerKiller` fire when the overlapping hazard is a child object, while avoiding a full room scan for every collision-event owner.
- Runtime-core lowered execution now emits block-level execution trace diagnostics and unsupported lowered-runtime diagnostics with the current room, tick, block id, object name, event tag, and runtime instance id. Unsupported statements are reported by lowered statement kind, and unsupported function calls are reported both when they appear as statements and when they appear inside evaluated expressions. The developer CLI can aggregate these with `cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --select-room <room_id> --ticks <n>`.
- Runtime-core expression evaluation now covers the Dife-critical helpers `abs()`, `floor()`, `random()`, `choose()`, `string()`, `instance_number()`, `instance_place()`, object-name `distance_to_object()`, object-name `collision_line()`, current `room`, named room constants such as `rSelectStage`, `instance_create()` expression references, and the world-step numlock pair `keyboard_get_numlock()` / `keyboard_set_numlock(off)`. `collision_line()` returns a hit instance id or GM `noone` (`-4`), `instance_place()` returns the hit instance id or `noone`, and numeric truthiness now follows GM/OpenGMK behavior (`number >= 0.5`) so `noone` is false in conditionals. Lowered `for` and `repeat` loops now execute with iteration guards, including assignment-shaped `for` init/step expressions, scoped iterator locals, loop bodies, and transition/reset interruption. Parser lowering preserves compound assignment steps such as `for(i = 0; i <= 100; i += 1)` as executable assignment-shaped step expressions instead of non-mutating `i + =1` expressions. This `instance_number()` slice is enough for the current Dife `playerShoot()` gate `if(instance_number(bullet) < 4)` to run on the lowered path, and the combined lowering/runtime fixes are now enough for the current Dife bullet self-destroy path `if(place_meeting(x+hspeed,y,block)){ a=instance_place(...); if(a.object_index=block) instance_destroy(); ... }` to execute on the lowered path. Local diagnostics against `sampleroom01` (`room_id = 143`) with `cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --preselect-ticks 2 --select-room 143 --ticks 80 --press-keys 90` now report `runtime-instance-created` for `bullet` at tick `3` and `runtime-instance-destroyed` from `object:2:event:3:0` at tick `7`, replacing the earlier alarm-only `bullet` expiry at tick `42`. Local diagnostics against `sampleroom01` with `--preselect-ticks 2 --select-room 143 --ticks 240 --press-keys 16 --trace-player --trace-every 20`, plus direct checks against `sampleroom03` (`room_id = 146`) and `room151` (`room_id = 151`), report empty `runtime_blockers` lists rather than per-tick `runtime-for-iteration-limit` or `keyboard_get_numlock` warnings from `world` step.
- The same `runtime-diagnostics` command can now collect headless player behavior traces without adding a separate CLI tool. Use `--trace-player --trace-every <n>` to add `trace_summary` plus a sampled `player_trace` array to the diagnostics JSON, or add `--trace-output <path>` for longer runs. `trace_summary` records first/last comparable frames, coordinate ranges, peak absolute speeds, sample count, and room segments for quick behavior-baseline comparison. Each trace row records room/tick, player object and runtime id, position, velocity, alive state, grounded/jump phase state, active input trace, and diagnostic count. This is the first command-line behavior-validation surface for comparing Dife movement and jump trajectories after unsupported runtime blockers are empty.
- `runtime-diagnostics` now also supports scripted input playback through `--input-script <path>`, using a JSON `ticks` array with per-tick `press_keys`, `hold_keys`, and `release_keys`. Script `tick` values are relative to the main diagnostics run after `--preselect-ticks` warmup and manual room selection, so `tick: 0` applies to the first tick of the selected diagnostic window. This is the current command-line path for reproducing multi-phase runtime behavior without relying on browser-only hand input.
- The same diagnostics JSON now also exposes `runtime_events` for high-value lifecycle markers, currently including room changes, restart requests, player death, and runtime instance create/destroy events. Events keep their original diagnostic `message` and also expose parsed fields when present, including `room`, `from_room`, `to_room`, `tick`, `block_id`, `object`, `event_tag`, `runtime_id`, `x`, `y`, and `reason`. Treat this as a compact runtime timeline surface, not as a replacement for full browser integration telemetry.
- Runtime-render frames now support a small `drawText` command in addition to sprite/background/tile/rect drawing. Runtime rendering now respects instance-level `visible`, `sprite_index`, `image_index`, `image_xscale`, and `image_yscale` values while preserving the current player facing-left mirror behavior. For the current Dife room151 death path, runtime-core executes the package script far enough to play `sndDeath`, create `bloodEmitter2`, execute the emitter Step loop, create and move `blood2` particles, create `GAMEOVER`, and wait until reset reloads the room. The earlier runtime-owned red `GAME OVER` fallback overlay has been removed so death presentation comes from package-owned resources.

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
- many sample-specific spawn chains still do not run
- traps do not trigger
- doors do not open
- score / state logic does not run
- broader `with` parity still depends on the remaining expression, lifecycle, and instance-id semantics even though the current lowered runtime slice now switches object/self/other/all execution context

### 2. Audio

Background music and sound effects now have a first browser-hosted path.

Runtime-core resolves package sound identifiers for `sound_play()`, `sound_loop()`, `sound_stop()`, `sound_stop_all()`, and `sound_isplaying()` and dispatches them through `RuntimeAudioHost`. The browser runtime-web path now forwards those host calls through WASM imports to a minimal Web Audio host that loads package sound resources, plays one-shot sounds, loops sounds, queries active sound state, stops active loops, and stops all active sounds.

Remaining audio gaps include browser autoplay/user-gesture handling, volume/pan/mixing controls, channel/priority semantics, and broader GM sound API coverage.

Deferred backlog: Dife background music currently exposes a create-time host-dispatch gap, not a Web Audio decode/playback gap. The package contains room-instance create blocks such as `sound_loop(track01)`, `sound_loop(PPPPPP)`, and `sound_stop_all()`, but room construction and create-time bootstrap logic are still largely hostless. Step-time sound effects can already reach `RuntimeAudioHost`, which is why jump SFX can play in the browser while BGM does not start when entering rooms whose music is launched from instance creation code. Keep this deferred until the main runtime gameplay path is more complete, unless `sound_isplaying()` or another sound query is proven to gate non-audio gameplay state.

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
- `instance_create()` now works on the shared lowered runtime event path and as an expression that returns a created-instance reference; follow-up member assignments are applied after the new instance's `Create` event without suppressing duplicate object instances
- collision events can now participate in this lifecycle chain by destroying their owner through lowered `instance_destroy()`, including parent-target collision events such as Dife's `playerKiller` path
- the current Dife room151 death path now reaches the package-owned `killPlayer` script branch, plays `sndDeath`, creates `bloodEmitter2`, executes the emitter Step loop, creates moving `blood2` particles, creates `GAMEOVER`, writes `DeathTime`, and waits for reset before room reload clears those instances. Remaining death-presentation gaps are now about finer sprite animation, draw handling, particle/audio parity, and broader sample coverage rather than the basic death/reset lifecycle.

For current planning purposes, `keyboard`, `collision`, `alarm`, and room-start handling should be treated as part of the first IWanna-critical lifecycle slice rather than as optional polish. Keyboard, alarm, room-start, and the current collision dispatch slice now exist in runtime-core; broader lifecycle parity still depends on more GM event categories and Draw-event execution.

## Important Missing

These do not always block booting, but they block core IWanna fidelity and make many rooms behave incorrectly.

### 7. Physics Precision

Current movement still uses some hardcoded defaults such as `RUN_SPEED` and fallback jump values, but jump is no longer a fixed-height placeholder, and motion assignments / axis deltas no longer lose GM8 fractional values on write or movement.

The runtime already has a hardcoded player movement baseline and per-instance `hspeed` / `vspeed` fields, plus explicit jump-phase state for hold, cut, and landing-reset behavior, but not a general GM8-style object-driven physics model.

Missing pieces include:

- per-object `friction`
- per-object `gravity`
- broader GM motion variables beyond the current `hspeed` / `vspeed` / `speed` / `direction` / `gravity` slice
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
The browser can now start from the original `rInit` order and advance toward title/menu/select rooms through lowered room logic, but full playability still depends on Draw-event text/sprite behavior, general file/save API parity beyond the currently proven minimal `file_bin_*` slice, and broader menu object logic.

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

1. use runtime unsupported diagnostics, block-level trace output, headless player traces, and browser timing telemetry to rank the active Dife path before adding more GM helpers
2. keep the shared lowered parser contract stable except where gold-sample evidence requires targeted expansion
3. headless OpenGMK-derived runtime extraction behind narrow host traits
4. browser WASM host integration for that runtime core
5. audio, animation, and broader lifecycle coverage after the runtime can execute trustworthy semantics

Current gameplay-blocker workflow:

1. Validate the local package before debugging browser symptoms: `cargo run -p iwm-cli -- validate-package --input .\runtime\public\packages\sample`.
2. Run targeted diagnostics against the active Dife room. If `runtime_blockers` is non-empty, rank by first room/tick/block/object and add the smallest runtime/parser slice needed for that proven path.
3. If `runtime_blockers` is empty, switch from unsupported-helper work to behavior validation. Capture `--trace-player` with controlled inputs, compare `trace_summary` first for quick drift detection, then inspect full `player_trace` rows for room/tick, `x/y`, `hspeed/vspeed`, alive state, grounded state, jump phase, active keys, and room transitions against browser observations or a reference run.
Use `--input-script` when the behavior depends on more than a single one-tick press or an always-held key, and use `runtime_events` to quickly spot whether a restart, room change, death, or runtime instance churn happened on the expected frame. The current checked-in Dife input scripts live under `docs/notes/runtime-scenarios/` and cover room 143 tap jump, held jump, release cut, right movement, and shoot, plus room 151 rightward hazard death and fallback `R` reset.
4. For browser-only problems, use the shell frame timings to separate runtime-core work (`tick` / `step` / `collision` / `renderSubmit`) from snapshot serialization and canvas rendering. Treat sustained large-room frame spikes as a performance blocker only after unsupported diagnostics are clean.
5. Promote a missing API or lowered construct only when it is tied to spawn, movement, death/reset, room transition, savepoint, camera, or another visible gameplay failure on the gold-sample path. Keep BGM create-time dispatch in the audio backlog unless it gates gameplay state.
