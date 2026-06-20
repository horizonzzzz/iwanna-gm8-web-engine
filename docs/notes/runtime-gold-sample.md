# Runtime Gold Sample

> **Current status note:** This is an active runtime-priority document, not a historical note.
>
> When the primary gold sample, local package availability, or proven blocker list changes, update this file in the same change.

This note keeps both the long-lived validation target and the current blocker audit for the WASM-first path.

Important local-path note:

- these sample paths are local development paths, not tracked sample binaries
- `runtime/public/packages/sample/` is the intended local output path for a generated smoke package, not a tracked artifact
- a fresh clone keeps `runtime/public/packages/` empty except for `.gitkeep`

## Primary Gold Sample

**Path:** `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

Why it still matters:

- it remains the intended first sample for validating boot, movement, death/reset, and room transition semantics
- it is the sample that should decide whether parser/package/runtime gaps are actually on the critical gameplay path
- it should stay ahead of secondary samples when priorities conflict

Important local-environment note:

- the local sample corpus may differ between development machines
- if this exact path is absent in the current environment, use the closest available `gm8-core` sample for local package-smoke verification
- that local fallback does not change the repository's intended gold-sample target or repo-wide contract

## Repo-Local Runtime Package

- `runtime/public/packages/sample/`

This is the intended local browser smoke target after generating a package from the gold sample. The repo does not currently ship a checked package artifact there.

## Blockers By Layer

- Parser/package availability: a fresh clone does not include `runtime/public/packages/sample/`, so the first gold-sample smoke prerequisite is still a local `build-package` run against `IWBT_Dife`.
- Package-contract validation: after generating `runtime/public/packages/sample/`, run `cargo run -p iwm-cli -- validate-package --input .\runtime\public\packages\sample` before treating browser symptoms as runtime-semantic failures. As of the current local IWBT_Dife smoke, the generated sample validates successfully; hidden title/menu background layers no longer block validation because they are not drawn by the current renderer contract.
- Runtime-core semantic gap: the remaining meaningful gaps are the ones that still block movement, collision, death/reset, or room transition after the WASM runtime has already booted and drawn a room.
- Wasm/web host gap: the host telemetry path exists, but gold-sample-specific runtime claims still require evidence from a locally generated `sample/` package.
- Shell-only issue: the default package path remains `/packages/sample`, so missing local artifacts should surface as explicit load errors rather than being mistaken for runtime-semantic failures.

Important validation note:

- because local sample inventories differ across machines, local gold-sample smoke should be treated as environment evidence, not as the only repository-level proof that a parser/runtime/package contract still holds
- stable repository fixtures and package-contract validation should catch structural drift before gold-sample browser debugging is needed
- the current runtime slice already covers alarm dispatch, held/press/release keyboard dispatch, and parent-aware event lookup, so the next gold-sample blockers should be judged against the remaining runtime gap rather than those already-covered slices
- parser-built room order now boots the sample from `rInit` and orders the initial chain as `rInit -> rTitle -> rMenu -> rSelectStage`, so title/menu/select navigation should now be debugged as runtime logic rather than by manually selecting those rooms first
- runtime-core no longer injects fallback players into menu-like rooms; player appearance now depends on explicit room instances or supported original spawn logic such as `other:room-start`
- jump is no longer a fixed-height placeholder in repository fixtures, and runtime-core now preserves fractional vertical jump movement for the sample's `jump=8.5` / `gravity=0.4` path; the remaining gold-sample jump work is numeric calibration of tap, hold, release-cut, double-jump, and landing-reset behavior against `IWBT_Dife`
- the shell/runtime snapshot path now exposes grounded plus jump-phase trace flags for the player, which makes browser-side hand-feel debugging easier but does not change the remaining semantic blocker: the gold sample still needs its own player movement path executed accurately

## Sample Audit

### IWBT_Dife

- Intended package path: `runtime/public/packages/sample/`
- Boot room: current local generated package boots from `rInit` (`room_id = 2`) through `manifest.default_room_id`
- Frame draws: the shell and WASM bridge can render telemetry and runtime-core now emits basic Draw-event text commands, but gold-sample-specific frame proof still depends on local package generation
- Player appears: not repo-proven on a tracked artifact
- Movement works: not yet verified as a dedicated browser assertion
- First blocking warning or missing behavior: title/menu/select can now be reached through the original room-order path, difficulty/select labels can use basic Draw-event text commands, and the Dife-critical `DeathTime` binary-file path plus room143 S-key `savePoint` path now have minimal runtime support. Full usability still depends on broader Draw-event rendering, menu logic, and general save/file API parity beyond the currently proven file slice.
- Sprite collision metadata is now present in the parser-emitted package contract as aggregated bbox bounds plus gm8exe-derived `collision_masks`; current runtime pixel checks use the first available sprite mask, while animated per-frame mask selection remains deferred
- Large-room view behavior: local generated package rooms such as `rStage01`
  preserve a `2400x1824` room with one visible `800x600` view; runtime-core now
  submits an `800x600` frame and can move the active view through the sample's
  `camera` object `view_xview` / `view_yview` logic.
- Runtime unsupported diagnostics are now available on the lowered execution path. After adding runtime-core support for `abs()`, `floor()`, `random()`, `random_range()`, `choose()`, `string()`, text concatenation through binary `+`, GML `div` / `mod`, host-backed `file_exists()` / `file_delete()`, `instance_number()`, `instance_place()`, object-name `distance_to_object()`, object-name `collision_line()`, current `room`, named room constants, lowered `for` / `repeat` loops, `instance_create()` expression references plus member writes, numeric object-id `instance_create()` targets, common DnD FUNCTION action lowering (`action_set_alarm`, `action_create_object`, `action_kill_object`), `event_inherited()` for parent event dispatch, create-time `instance_destroy()` for room-placed instances, the Dife-critical GM numeric comparison semantics, non-player GM motion variables, the Dife BGM-critical sound subset (`sound_play()` / `sound_loop()` / `sound_stop()` / `sound_stop_all()` / `sound_isplaying()`), the Dife `DeathTime` and room143 `savePoint` `file_bin_*` paths, nested binary-file reads inside arithmetic expressions, basic Draw-event text commands, and the world-step numlock pair (`keyboard_get_numlock()` / `keyboard_set_numlock(off)`), the current scripted local runs against `sampleroom01` (`room_id = 143`) report empty `runtime_blockers` lists for tap jump, held jump, release cut, right movement, shoot, and S-key savePoint activation. Direct local diagnostics against `sampleroom03` (`room_id = 146`) and `room151` (`room_id = 151`) also report empty `runtime_blockers` lists for the current scripted baseline set. The Dife shoot-path script `docs/notes/runtime-scenarios/dife-room143-shoot.json` with `cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --preselect-ticks 2 --select-room 143 --ticks 80 --trace-player --trace-every 20 --input-script docs\notes\runtime-scenarios\dife-room143-shoot.json` reports structured `runtime_events`: `runtime-instance-created` for `bullet` in room `143` at tick `3` (`x=81`, `y=567.4`, runtime id `172` in the current local run) and `runtime-instance-destroyed` for the same `bullet` from block `object:2:event:3:0` at tick `7`, proving that the generated Dife bullet path moves forward, probes `block`, and self-destroys from its lowered `step` logic instead of only expiring later through `alarm[0]`. The repository regression `real_sample_s_key_savepoint_writes_save_file_and_spawns_feedback` directly drives S on a live room143 savePoint, proves `save1` is written, verifies package-owned feedback objects `object808`, `object809`, and `object819` are created, then ticks the helper alarm long enough to prove the package recreates a live `savePoint` and destroys the helper without unsupported lowered-runtime diagnostics. The repository regression `real_sample_room147_s_key_savepoint_respawns_at_activated_position` covers `rStage01` (`room_id = 147`) with many savePoints and proves the activated savePoint at `(864, 1120)` reappears at the same coordinates after the package-owned helper alarm. The repository regression `real_sample_r_load_after_s_save_restores_saved_player_position` covers the same save path followed by the package-owned R load path, proving the restored player uses the exact saved position after `game_restart()` boots through the init room, creates the persistent player, and jumps back to the saved room. Browser manual room selection remains available for diagnostics, but it no longer writes test difficulty through a WASM global override; direct-room checks that depend on difficulty should either enter through the package-owned difficulty menu or rely on package bootstrap state. Local browser checks need a package rebuilt with the current parser, otherwise older ignored `runtime/public/packages/sample` artifacts can still contain stale lowered draw blocks.
- Behavior trace is now part of the same diagnostics command rather than a separate tool. For movement and jump validation, run a targeted room with a stable input script, for example `cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --preselect-ticks 2 --select-room 143 --ticks 240 --trace-player --trace-every 20 --input-script docs\notes\runtime-scenarios\dife-room143-hold-jump.json`, then compare `trace_summary` first for first/last frames, coordinate range, peak absolute speeds, sample count, and room segments. Use full `player_trace` rows for player `x/y/hspeed/vspeed`, grounded state, jump active/hold/cut state, alive state, active keys, and room/tick transitions after the summary shows a drift or needs explanation. The `--preselect-ticks` warmup matters for Dife because the Web runtime normally lets default-room bootstrap logic initialize globals such as `global.jumpbutton` before entering a gameplay room; input-script tick values are relative to the selected diagnostics window, so `tick: 0` applies after that warmup and room selection. Current local `sampleroom01` scripted trace baselines with `--trace-every 20` are: tap Shift over 240 ticks gives `x=81.0`, `min_y=558.535`, `max_y=567.445`, `max_abs_vspeed=3.155`, `sample_count=12`, and empty blockers; held Shift over 240 ticks gives `x=81.0`, `min_y=482.8`, `max_y=567.3`, `max_abs_vspeed=6.7`, `sample_count=12`, and empty blockers; release-cut at tick `8` over 240 ticks gives `x=81.0`, `min_y=511.95`, `max_y=567.33`, `max_abs_vspeed=1.615`, `sample_count=12`, and empty blockers; held Right over 120 ticks gives `x=135.0..154.0`, `y=567.4`, `max_abs_hspeed=3.0`, `sample_count=6`, and empty blockers. Treat these as comparison baselines, not final proof of native-feel parity.
- Current local death scripted baselines use `room151` (`room_id = 151`) because it has a stable rightward hazard path. `docs/notes/runtime-scenarios/dife-room151-death-right.json` with `--preselect-ticks 2 --select-room 151 --ticks 180 --trace-player --trace-every 20` reports empty blockers and a first death at room `151`, tick `50`, object `player`, `x=257`, `y=243.4`, `reason=hazard`. The same event window then shows package-owned death feedback: `bloodEmitter2` and `GAMEOVER` are created at tick `50`, the emitter Step loop creates moving red `blood2` particles before the emitter's alarm destroys it, and no room restart occurs during the 180-tick run. A longer local debug run of the same script with `--ticks 1200 --trace-output target\runtime-dife-room151-1200-final.json` reaches tick `1202`, reports `287` `blood2` creates, `0` blockers, and no collision trace diagnostics. Raw `R` is now treated as package keyboard input in the browser and the real-sample core path, so the older room151 `R` reset scripts should be treated as historical fallback-host checks rather than current browser behavior. The repository regression for death feedback binds runtime restart to a non-package key before pressing it, proving explicit host restart still returns the player to room151 and clears `GAMEOVER` / `bloodEmitter2` through room reload.

The diagnostics command can collect a pre-run trace row when manual room selection settles the selected room and the current runtime tick is divisible by `--trace-every`. With the current Dife scripted baseline shape (`--preselect-ticks 2 --trace-every 20`), sampled rows start at tick `20`.

Critical-path `runtime-missing-source-lowering:*` warnings:

- do not escalate broad `runtime-missing-source-lowering:*` lists from `analysis.json` as critical until one is tied to spawn, movement, death/reset, or room transition failure on the actual locally generated gold-sample path

## Current Validation Behaviors To Prove

For `IWBT_Dife`, Phase 4 still needs to prove:

- package loads successfully
- the runtime core boots the intended first playable room
- the browser host can drive a stable 60 Hz auto-tick loop with pause/resume control for hand-feel validation
- player movement and collision match the runtime-core semantic slice being implemented
- variable-height jump, release cut, and landing reset match the intended `IWBT_Dife` trajectory closely enough to use as the runtime jump baseline
- broader collision dispatch and remaining room/lifecycle semantics still behave as expected now that keyboard and alarm slices are covered
- at least one room transition works through the WASM path
- diagnostics stay explicit when unsupported logic, externals, or host gaps are hit

## Notes

- `runtime-missing-source-lowering:*` warnings are not uniformly urgent; only warnings proven to sit on the first-room, spawn, movement, death/reset, or transition path should drive immediate runtime work.
- Do not treat the absence of `runtime/public/packages/sample/` in a fresh clone as evidence of a runtime failure; it is a local artifact prerequisite.

## References

- Package format: `docs/notes/package-format-v1-runtime.md`
- WASM runtime gap analysis: `docs/notes/runtime-wasm-gap-analysis.md`
- Runtime vendor reference map: `docs/notes/runtime-vendor-reference-map.md`
- Design spec: `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
