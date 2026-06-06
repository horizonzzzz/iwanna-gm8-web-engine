# Package Format V1 Runtime

> **Current status note:** This is the active package-format note.
>
> The older V0 package note is obsolete and should not be used as the current contract.

Current emitted runtime package directory contents:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `logic.raw.json`
- `logic.lowered.json`
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
- the current parser-side lowering is not considered semantically sufficient for long-term gameplay execution
- if the WASM-hosted runtime later requires a richer execution input, this format may evolve again
- until then, these outputs remain useful for package inspection, diagnostics, and shell bring-up

Included in this phase:

- browser-ready sprite exports
- sprite exports now include `bbox_left`, `bbox_right`, `bbox_top`, and `bbox_bottom` collision bounds plus optional `collision_masks` sourced from the parser's OpenGMK sprite collision metadata
- browser-ready background exports
- audio file exports
- normalized room instance placements with runtime categorization hints
- parser-normalized GM room order in `manifest.room_order`
- normalized object event table with event tags and collision target ids for dispatch
- logic envelope in `scripts.ir.json` with executable/source-only distinction
- raw parser-owned GML preservation in `logic.raw.json`
- structured parser-owned lowered logic in `logic.lowered.json` for the current IWanna-critical subset
- the current lowered contract also preserves common comment stripping, `var` declarations, unary expressions, and `return` statements on the current critical path
- control-flow heads in `logic.lowered.json` are represented as lowered expressions so the WASM bridge can deserialize them directly
- runtime categorization: hazard, checkpoint, player-controlled hints

## Current Shell Integration

Today the browser shell expects a package directory under `runtime/public/packages/<name>/` and loads:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `logic.raw.json`
- `logic.lowered.json`
- `analysis.json`
- `resources/index.json`

The default shell input is `/packages/sample`, which corresponds to `runtime/public/packages/sample/`.

The current `iwm-runtime-web` bridge still boots from the normalized runtime payload; the raw and lowered logic files are parser-side artifacts used to preserve and prepare GM8 logic for later runtime consumption.
The browser shell also loads `logic.raw.json` and `logic.lowered.json` today so diagnostics and future runtime-facing tooling can inspect parser-owned logic without reopening the original GM8 executable.

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
- Manifest with default room, source room order, and compatibility metadata

### Runtime Contract Invariants

The current package format relies on a small set of identity and reference invariants that runtime consumers should treat as part of the contract, not as best-effort hints.

Important current invariants:

- `rooms[*].instances[*].object_id` refers to `objects[*].id`, not to the array position of an object entry
- `objects[*].sprite_index` refers to `resources.index.json -> sprites[*].id` when non-negative
- room background and tile references refer to `resources.index.json -> backgrounds[*].id`
- `manifest.room_order` is optional for older packages, but when present each room id must resolve to `rooms[*].id`; parser-built packages use this order for `default_room_id` and runtime `room_goto_next()` semantics
- room, instance, and object event block ids should resolve consistently across `scripts.ir.json`, `logic.raw.json`, and `logic.lowered.json`
- sprite resource collision bounds are emitted in `resources/index.json` for each sprite record; the parser also emits `collision_masks` and `per_frame_collision_masks` from gm8exe collision maps so runtime consumers can perform pixel-level checks after bbox broad-phase filtering
- runtime consumers should validate cross-file references explicitly instead of silently assuming contiguous ids

This matters because normalized package ids may remain sparse even when the emitted JSON arrays are dense. Runtime code must resolve identities by `id` rather than by array offset.

The repository now has a structural validator in `crates/iwm-runtime-model/` exposed as `validate_runtime_package()` and through:

```powershell
cargo run -p iwm-cli -- validate-package --input .\runtime\public\packages\sample
```

The validator is contract-oriented, not semantic. It checks package shape and cross-file references before browser smoke, while runtime behavior validation remains in `iwm-runtime-core`, `iwm-runtime-web`, and browser tests.
Room background resource validation follows the current runtime drawing contract: visible room background layers and all tiles must resolve to exported background resources; hidden room background layers are preserved but do not currently block validation because neither runtime-core nor the browser static renderer draws them.

### Currently Executable Action-List Subset

The following `action-list` script blocks can be executed by the browser runtime:

- Basic variable reads and writes for instance-local state
- Simple arithmetic operations
- Conditional branches (if/else)
- Movement-related action calls (when implemented in logic runner)

`LogicBlock.executable_action_count` indicates how many actions can run without GML lowering.

This is currently useful for diagnostics and shell validation, but it is not the intended long-term execution architecture now that the project has adopted a WASM-first runtime strategy.

### Parser-Owns Raw And Lowered Logic

- `logic.raw.json` preserves the original GML source text and ownership metadata for room, instance, object event, script, trigger, and timeline logic
- `logic.lowered.json` holds the parser-owned lowered contract for current critical-path expressions and statements such as calls, assignments, member access, index access, binary expressions, `var` declarations, `return` statements, and structured control-flow heads
- runtime should treat these files as the bridge between `gm8exe` extraction and executable runtime semantics, not as a separate public API for end users
- current repository direction assumes that `logic.lowered.json` must keep moving toward a structurally correct runtime-facing contract; any remaining raw fallback is transitional diagnostics, not the intended steady-state execution contract
- for the active Phase 4 route, parser work should converge on real callable structure for the IWanna-critical subset even if full general GML support remains out of scope

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
- follow parser-provided `manifest.room_order` for package boot and `room_goto_next()`
- return formatted diagnostics
- clear host edge bits after each tick so one-shot keyboard input does not repeat across bridge frames

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
- Advanced GML functions not yet in the supported subset
- Particle systems, surfaces, and advanced drawing
- Menu systems and save/load functionality
- DLL semantics and external function calls
- Advanced physics beyond the current bbox broad-phase plus sprite-mask pixel collision path
- high-fidelity continuous browser host timing and play-loop controls

## Route Decision Implication

This package note now reflects the selected development route:

- runtime semantics should accumulate in the OpenGMK-derived WASM path
- parser semantics should accumulate in project-owned extraction and lowering code
- the package should keep serving as the seam between those two tracks

That means:

- do not keep adding package fields whose only purpose is to support a project-owned TS gameplay engine
- prefer fields that help a headless/WASM runtime execute real semantics or explain why it cannot
- preserve `logic.raw.json` and `logic.lowered.json` as diagnostic and transition artifacts until a stronger parser-owned execution contract is proven

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
