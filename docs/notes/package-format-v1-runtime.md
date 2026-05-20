# Package Format V1 Runtime

Current emitted runtime package directory contents:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `analysis.json`
- `resources/index.json`
- `resources/sprites/...`
- `resources/backgrounds/...`
- `resources/audio/...`

This package is runtime-consumable but still phase-limited.

Important direction note:

- this package remains the current browser-shell input format
- the current TypeScript runtime consumption path is transitional
- the current WASM bridge also boots from this normalized JSON package today
- if the WASM-hosted runtime later requires a richer execution input, this format may evolve again
- until then, these outputs remain useful for package inspection, diagnostics, and shell bring-up

Included in this phase:

- browser-ready sprite exports
- browser-ready background exports
- audio file exports
- normalized room instance placements with runtime categorization hints
- normalized object event table with event tags for dispatch
- logic envelope in `scripts.ir.json` with executable/source-only distinction
- runtime categorization: hazard, checkpoint, player-controlled hints

## Current Shell Integration

Today the browser shell expects a package directory under `runtime/public/packages/<name>/` and loads:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `analysis.json`
- `resources/index.json`

The default shell input is `/packages/sample`, which corresponds to `runtime/public/packages/sample/`.

The current `iwm-runtime-web` bridge boots from the same normalized JSON payload after the frontend aggregates these files.

## Current Execution Status

The execution notes below describe the current package contract and shell/runtime bring-up status. Any TypeScript execution notes are transitional implementation status, not the final long-term engine direction.

### Runtime-Consumable Static Data

- Room dimensions, backgrounds, views, and instance placements
- Instance-level `is_solid`, `is_hazard`, `is_checkpoint` flags
- Room-level `playable` flag and `transition_targets` hints
- Object definitions with sprite references and event tables
- Object-level `is_hazard`, `is_checkpoint`, `is_player` hints
- Event entries with normalized `event_tag` for runtime dispatch
- Resource index with paths to browser-loadable assets
- Manifest with default room and compatibility metadata

### Currently Executable Action-List Subset

The following `action-list` script blocks can be executed by the browser runtime:

- Basic variable reads and writes for instance-local state
- Simple arithmetic operations
- Conditional branches (if/else)
- Movement-related action calls (when implemented in logic runner)

`LogicBlock.executable_action_count` indicates how many actions can run without GML lowering.

This is currently useful for diagnostics and shell validation, but it is not the intended long-term execution architecture now that the project has adopted a WASM-first runtime strategy.

### Current WASM Bridge Status

The current `iwm-runtime-web` bridge can now:

- accept the normalized runtime package as JSON
- boot a headless runtime-core instance
- accept browser-submitted keyboard input snapshots
- return runtime snapshots
- return browser-consumable frame snapshots
- advance deterministic ticks
- reset the runtime
- switch rooms by room id
- return formatted diagnostics

It does **not** yet provide:

- audio playback
- DLL/external support
- gameplay-fidelity parity with OpenGMK runner semantics
- a fully continuous real-time gameplay loop in the shell

### Current Browser Host Status

The current browser-hosted runtime flow is:

- frontend package loader aggregates the normalized runtime package
- `iwm-runtime-web` boots and ticks against that normalized payload
- the browser shell submits keyboard input snapshots to the bridge
- the bridge returns frame commands for the active room
- `runtime/` draws those commands onto the existing canvas using `resources/index.json`

### Still Deferred / Unsupported

- `source-only` script blocks that require GML lowering
- Advanced GML functions not yet in the supported action subset
- Particle systems, surfaces, and advanced drawing
- Menu systems and save/load functionality
- DLL semantics and external function calls
- Complex collision masks and advanced physics
- high-fidelity continuous browser host timing and play-loop controls

### Analysis Warnings

Current `analysis.json` warnings include actionable categories:

- `runtime-missing-source-lowering:<block_id>` - source-only blocks requiring GML lowering
- `runtime-unsupported-event:<event_tag>` - event types not yet supported (e.g., triggers, user events)
- `runtime-unsupported-action:<fn_name>` - actions not yet implemented (e.g., file_*, sound_*, window_*)
- `logic-execution-not-yet-implemented` - general execution placeholder

These warnings still guide parser and shell diagnostics work, but gameplay-runtime prioritization now belongs to the WASM-first runtime plan.

### Event Tag Normalization

Event entries include a normalized `event_tag` for runtime dispatch:

| Event Type | Tag Format | Example |
|------------|-----------|---------|
| Create | `create` | `create` |
| Destroy | `destroy` | `destroy` |
| Alarm | `alarm:<n>` | `alarm:0`, `alarm:5` |
| Step | `step`, `step:begin`, `step:end` | `step` |
| Collision | `collision` | `collision` |
| Keyboard | `keyboard:<key>` | `keyboard:a` |
| Mouse | `mouse:left`, `mouse:right`, etc. | `mouse:left` |
| Other | `other:<name>` | `other:outside`, `other:no-health` |
| Draw | `draw` | `draw` |
| Key Press | `keypress:<key>` | `keypress:a` |
| Key Release | `keyrelease:<key>` | `keyrelease:a` |
| Trigger | `trigger:<n>` | `trigger:0` |
