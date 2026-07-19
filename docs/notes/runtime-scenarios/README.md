# Runtime Scenario Input Scripts

These files are small `iwm-cli runtime-diagnostics --input-script` fixtures for
local behavior checks. They contain only key timing data, not sample game assets.

Most detailed behavior scripts target the local `IWBT_Dife` package at:

- `runtime/public/packages/sample/`

Expected command shape:

```powershell
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --preselect-ticks 2 --select-room 143 --ticks 240 --trace-player --trace-every 20 --input-script docs\notes\runtime-scenarios\dife-room143-hold-jump.json
```

Script `tick` values are relative to the main diagnostics run after any
`--preselect-ticks` warmup and manual room selection. A script entry at `tick: 0`
therefore applies to the first tick after the selected room has been settled.

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
