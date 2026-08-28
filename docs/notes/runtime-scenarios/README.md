# Runtime Scenario Input Scripts

These files are small `iwm-cli runtime-diagnostics --input-script` fixtures for
local behavior checks. They contain only key timing data, not sample game assets.

Most detailed behavior scripts target the local `IWBT_Dife` package at:

- `runtime/public/packages/gm8-core/IWBT_Dife/`

Expected command shape:

```powershell
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\gm8-core\IWBT_Dife --preselect-ticks 2 --select-room 143 --ticks 240 --trace-player --trace-every 20 --input-script docs\notes\runtime-scenarios\dife-room143-hold-jump.json

# Capture structured collision pairs and the final 12 collision phases at death.
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\gm8-core\IWBT_Dife --preselect-ticks 2 --select-room 151 --ticks 180 --trace-collisions --trace-on-death --death-trace-window 12 --input-script docs\notes\runtime-scenarios\dife-room151-death-right.json --trace-output target\dife-room151-collision.json
```

Script `tick` values are relative to the main diagnostics run after any
`--preselect-ticks` warmup and manual room selection. A script entry at `tick: 0`
therefore applies to the first tick after the selected room has been settled.

`--trace-collisions` adds structured `collision_trace` entries with both
participants' runtime/object IDs, current and previous positions, bounds,
mask presence, solid/hazard flags, collision phase, and collision event block
IDs. `--trace-on-death` records `death_trace` entries, each retaining the last
`--death-trace-window` collision phases. The JSON output is portable evidence;
replaying it still requires the same locally generated package.

Scenario assertions may set `"no_player_death": true` to turn an input script
into a portable no-death regression without committing sample assets.

Current Dife scenarios:

- `dife-room143-tap-jump.json`
- `dife-room143-hold-jump.json`
- `dife-room143-release-cut.json`
- `dife-room143-move-right.json`
- `dife-room143-shoot.json`
- `dife-room151-death-right.json`
- `dife-room151-r-reset.json` (historical raw-key reference)

Sample-level regression and development baselines:

- `ariotrials-title-idle.json`
- `ariotrials-select-stage-player.json`
- `crimson-v1-title-idle.json`

The current Crimson L3 title baseline uses:

```powershell
cargo run -p iwm-cli -- runtime-scenario --input ".\runtime\public\packages\gm8-core\I wanna be the Crimson ver.1.0" --scenario .\docs\notes\runtime-scenarios\crimson-v1-title-idle.json --ticks 600
```

The room 151 raw-`R` reset scripts predate the browser keyboard change that
treats physical `R` as package-owned raw keyboard input. They are useful only as
historical raw-key references; current browser and real-sample checks should let
package `keypress R` logic run without adding a second shell reset. Use
`iwm-cli runtime-diagnostics --press-restart` for explicit semantic host restart
checks.
