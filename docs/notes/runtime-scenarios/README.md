# Runtime Scenario Input Scripts

These files are small `iwm-cli runtime-diagnostics --input-script` fixtures for
local behavior checks. They contain only key timing data, not sample game assets.

Current scripts target the local `IWBT_Dife` package at:

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
- `dife-room151-r-reset.json`

The room 151 reset script uses key code `82` (`R`) because that is the current
runtime host fallback reset key. That should not be interpreted as a fixed IWanna
rule: runtime-core now checks package/runtime globals such as
`global.restartbutton` and `global.resetbutton` before falling back to `R`.
