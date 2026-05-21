# Runtime Rust Modularization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modularize the heaviest runtime Rust crates by responsibility while preserving all current behavior, public APIs, test entrypoints, and browser/runtime contracts.

**Architecture:** Refactor `iwm-runtime-host`, `iwm-runtime-web`, and `iwm-runtime-core` internally, crate by crate, by extracting cohesive internal modules and restoring the current public crate surfaces through `lib.rs` re-exports. Keep the work strictly structural: no feature additions, no parser changes, no FFI renames, and no behavior fixes folded into the split.

**Tech Stack:** Rust workspace crates, Cargo test runner, existing runtime/browser tests, `serde`, `serde_json`, current WASM bridge and runtime shell

---

## File Structure

Planned files for this phase:

- Modify: `README.md`
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Create: `crates/iwm-runtime-host/src/types.rs`
- Create: `crates/iwm-runtime-host/src/traits.rs`
- Create: `crates/iwm-runtime-host/src/clock.rs`
- Create: `crates/iwm-runtime-host/src/input.rs`
- Create: `crates/iwm-runtime-host/src/render.rs`
- Create: `crates/iwm-runtime-host/src/audio.rs`
- Create: `crates/iwm-runtime-host/src/files.rs`
- Create: `crates/iwm-runtime-host/src/externals.rs`
- Create: `crates/iwm-runtime-host/src/diagnostics.rs`
- Create: `crates/iwm-runtime-host/src/headless.rs`
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Create: `crates/iwm-runtime-web/src/bridge_types.rs`
- Create: `crates/iwm-runtime-web/src/web_runtime_host.rs`
- Create: `crates/iwm-runtime-web/src/translate.rs`
- Create: `crates/iwm-runtime-web/src/result_store.rs`
- Create: `crates/iwm-runtime-web/src/ffi.rs`
- Create: `crates/iwm-runtime-web/tests/web_runtime_host.rs`
- Create: `crates/iwm-runtime-web/tests/bridge_json.rs`
- Create: `crates/iwm-runtime-web/tests/local_package_smoke.rs`
- Modify: `crates/iwm-runtime-core/src/lib.rs`
- Create: `crates/iwm-runtime-core/src/types.rs`
- Create: `crates/iwm-runtime-core/src/core.rs`
- Create: `crates/iwm-runtime-core/src/room_builder.rs`
- Create: `crates/iwm-runtime-core/src/room_transitions.rs`
- Create: `crates/iwm-runtime-core/src/movement.rs`
- Create: `crates/iwm-runtime-core/src/logic.rs`
- Create: `crates/iwm-runtime-core/src/render.rs`
- Create: `crates/iwm-runtime-core/src/diagnostics.rs`
- Create: `crates/iwm-runtime-core/src/helpers.rs`
- Create: `crates/iwm-runtime-core/tests/boot.rs`
- Create: `crates/iwm-runtime-core/tests/movement.rs`
- Create: `crates/iwm-runtime-core/tests/logic.rs`
- Create: `crates/iwm-runtime-core/tests/render.rs`
- Create: `crates/iwm-runtime-core/tests/transitions.rs`

Responsibilities:

- `iwm-runtime-host`: host contracts, runtime host types, and headless default host composition
- `iwm-runtime-web`: browser/WASM bridge models, translation, host wrapper, result storage, and FFI exports
- `iwm-runtime-core`: runtime package/state types, orchestration, room building, movement, lowered logic, rendering, and diagnostics
- `README.md`: record the modularized runtime crate layout for future contributors

### Task 1: Snapshot Current Runtime Verification Baseline

**Files:**
- Modify: none

- [ ] **Step 1: Run runtime-host crate tests and capture the current green baseline**

Run:

```powershell
cargo test -p iwm-runtime-host
```

Expected:

```text
test result: ok
```

- [ ] **Step 2: Run runtime-web crate tests and capture the current green baseline**

Run:

```powershell
cargo test -p iwm-runtime-web
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run runtime-core crate tests and capture the current green baseline**

Run:

```powershell
cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Run the runtime frontend tests to pin the current browser-facing contract**

Run:

```powershell
npm --prefix runtime test
```

Expected:

```text
all tests pass
```

### Task 2: Split `iwm-runtime-host` Into Internal Responsibility Modules

**Files:**
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Create: `crates/iwm-runtime-host/src/types.rs`
- Create: `crates/iwm-runtime-host/src/traits.rs`
- Create: `crates/iwm-runtime-host/src/clock.rs`
- Create: `crates/iwm-runtime-host/src/input.rs`
- Create: `crates/iwm-runtime-host/src/render.rs`
- Create: `crates/iwm-runtime-host/src/audio.rs`
- Create: `crates/iwm-runtime-host/src/files.rs`
- Create: `crates/iwm-runtime-host/src/externals.rs`
- Create: `crates/iwm-runtime-host/src/diagnostics.rs`
- Create: `crates/iwm-runtime-host/src/headless.rs`

- [ ] **Step 1: Add module declarations and stable re-exports to the host crate root**

Write `crates/iwm-runtime-host/src/lib.rs` as:

```rust
//! Host-boundary contracts for the WASM-first runtime path.
//!
//! This crate intentionally stays small. It defines the narrow host traits and
//! headless helpers needed for the first OpenGMK feasibility spike without
//! mirroring the full `gm8emulator` surface area.

mod audio;
mod clock;
mod diagnostics;
mod externals;
mod files;
mod headless;
mod input;
mod render;
mod traits;
mod types;

pub use audio::NoopAudioHost;
pub use clock::DeterministicClock;
pub use diagnostics::VecDiagnosticsHost;
pub use externals::RejectingExternalHost;
pub use files::MemoryFileHost;
pub use headless::HeadlessHost;
pub use input::SnapshotInputHost;
pub use render::NullRenderHost;
pub use traits::{
    RuntimeAudioHost, RuntimeDiagnosticsHost, RuntimeExternalHost, RuntimeFileHost,
    RuntimeHost, RuntimeInputHost, RuntimeRenderHost, RuntimeTimeHost,
};
pub use types::{
    ButtonState, ExternalSignature, ExternalValue, Rgba8, RuntimeButton, RuntimeDiagnostic,
    RuntimeDiagnosticLevel, RuntimeDrawCommand, RuntimeHostError, RuntimeHostErrorKind,
    RuntimeRenderFrame, RuntimeSoundMode, DEFAULT_TICK_RATE_HZ,
};
```

- [ ] **Step 2: Extract runtime host base types into `types.rs`**

Write `crates/iwm-runtime-host/src/types.rs` with the current definitions for:

- `DEFAULT_TICK_RATE_HZ`
- `RuntimeButton`
- `ButtonState`
- `Rgba8`
- `RuntimeDrawCommand`
- `RuntimeRenderFrame`
- `RuntimeSoundMode`
- `ExternalSignature`
- `ExternalValue`
- `RuntimeDiagnosticLevel`
- `RuntimeDiagnostic`
- `RuntimeHostErrorKind`
- `RuntimeHostError`
- `impl RuntimeHostError`
- `impl Display for RuntimeHostError`
- `impl Error for RuntimeHostError`

Use the current code exactly, changing only import paths as needed.

- [ ] **Step 3: Extract runtime host traits into `traits.rs`**

Write `crates/iwm-runtime-host/src/traits.rs` with the current trait definitions for:

- `RuntimeTimeHost`
- `RuntimeInputHost`
- `RuntimeRenderHost`
- `RuntimeAudioHost`
- `RuntimeFileHost`
- `RuntimeExternalHost`
- `RuntimeDiagnosticsHost`
- `RuntimeHost`
- blanket `impl<T> RuntimeHost for T`

Import `Path`, `PathBuf`, and the moved types from `crate::types`.

- [ ] **Step 4: Extract deterministic clock support into `clock.rs`**

Write `crates/iwm-runtime-host/src/clock.rs` with the current `DeterministicClock` struct and its `impl`, `Default`, and `RuntimeTimeHost` implementation.

- [ ] **Step 5: Extract input host state into `input.rs`**

Write `crates/iwm-runtime-host/src/input.rs` with the current `SnapshotInputHost` struct and its methods plus `RuntimeInputHost` implementation.

- [ ] **Step 6: Extract render/audio/files/externals/diagnostics/headless implementations into dedicated modules**

Move the existing code into:

- `render.rs` for `NullRenderHost`
- `audio.rs` for `NoopAudioHost`
- `files.rs` for `MemoryFileHost`
- `externals.rs` for `RejectingExternalHost`
- `diagnostics.rs` for `VecDiagnosticsHost`
- `headless.rs` for `HeadlessHost` and its trait implementations

Use the current behavior exactly.

- [ ] **Step 7: Recreate the existing host tests in the new module layout**

Preserve the current seven tests by placing focused `#[cfg(test)]` modules in the corresponding implementation files:

- clock test in `clock.rs`
- file host tests in `files.rs`
- render test in `render.rs`
- input test in `input.rs`
- externals test in `externals.rs`
- headless composition test in `headless.rs`

Do not change assertions.

- [ ] **Step 8: Run the host crate tests**

Run:

```powershell
cargo test -p iwm-runtime-host
```

Expected:

```text
test result: ok
```

- [ ] **Step 9: Commit the host modularization**

```bash
git add crates/iwm-runtime-host/src
git commit -m "refactor(runtime-host): split host internals by responsibility"
```

### Task 3: Split `iwm-runtime-web` Bridge Models, Translation, Result Storage, And FFI

**Files:**
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Create: `crates/iwm-runtime-web/src/bridge_types.rs`
- Create: `crates/iwm-runtime-web/src/web_runtime_host.rs`
- Create: `crates/iwm-runtime-web/src/translate.rs`
- Create: `crates/iwm-runtime-web/src/result_store.rs`
- Create: `crates/iwm-runtime-web/src/ffi.rs`

- [ ] **Step 1: Replace the web crate root with module declarations and stable exports**

Write `crates/iwm-runtime-web/src/lib.rs` as:

```rust
mod bridge_types;
mod ffi;
mod result_store;
mod translate;
mod web_runtime_host;

pub use bridge_types::{
    BridgeDrawCommand, BridgeFrameSnapshot, BridgePlayerSnapshot, BridgeSnapshot, WebInputState,
};
pub use ffi::{
    iwm_alloc, iwm_boot_json, iwm_diagnostics_json, iwm_frame_json, iwm_free,
    iwm_last_result_len, iwm_reset, iwm_select_room, iwm_set_input_json, iwm_snapshot_json,
    iwm_tick,
};
pub use web_runtime_host::WebRuntimeHost;
```

- [ ] **Step 2: Extract bridge-facing JSON models into `bridge_types.rs`**

Move the current definitions for:

- `WebInputState`
- `BridgePlayerSnapshot`
- `BridgeSnapshot`
- `BridgeDrawCommand`
- `BridgeFrameSnapshot`

Use the current serde attributes unchanged.

- [ ] **Step 3: Extract `WebRuntimeHost` behavior into `web_runtime_host.rs`**

Move the current:

- `WebRuntimeHost` struct
- `new`, `boot`, `boot_from_json`, `set_input`, `tick`, `reset`, `select_room`, `snapshot`, `diagnostics`, `frame_snapshot`, `host_frame_count`
- `Default` implementation

Import translation helpers from `translate.rs`.

- [ ] **Step 4: Extract snapshot and draw-command translation into `translate.rs`**

Move the current helper functions:

- `bridge_snapshot`
- `bridge_player_snapshot`
- `bridge_draw_command`
- `format_diagnostics`
- `diagnostic_level_label`
- `status_label`
- `format_core_error`

Preserve the current output strings and JSON field semantics exactly.

- [ ] **Step 5: Extract result-buffer and payload parsing logic into `result_store.rs`**

Move the current:

- `last_result_bytes`
- `store_result`
- `store_json_result`
- `store_error_result`
- `read_utf8_from_ptr`

Keep the current escaping behavior for JSON error strings unchanged.

- [ ] **Step 6: Extract exported WASM entrypoints into `ffi.rs`**

Move the current:

- `runtime_host`
- `iwm_alloc`
- `iwm_free`
- `iwm_last_result_len`
- `iwm_boot_json`
- `iwm_set_input_json`
- `iwm_tick`
- `iwm_reset`
- `iwm_select_room`
- `iwm_snapshot_json`
- `iwm_frame_json`
- `iwm_diagnostics_json`

Keep all existing function names and signatures unchanged.

- [ ] **Step 7: Run the web crate tests**

Run:

```powershell
cargo test -p iwm-runtime-web
```

Expected:

```text
test result: ok
```

- [ ] **Step 8: Commit the web bridge modularization**

```bash
git add crates/iwm-runtime-web/src
git commit -m "refactor(runtime-web): split bridge host and ffi modules"
```

### Task 4: Reorganize `iwm-runtime-web` Tests By Responsibility

**Files:**
- Create: `crates/iwm-runtime-web/tests/web_runtime_host.rs`
- Create: `crates/iwm-runtime-web/tests/bridge_json.rs`
- Create: `crates/iwm-runtime-web/tests/local_package_smoke.rs`
- Modify: `crates/iwm-runtime-web/src/lib.rs`

- [ ] **Step 1: Add a shared sample package builder inside each focused integration test file**

Create:

- `tests/web_runtime_host.rs` for host lifecycle, motion, reset, diagnostics, and frame snapshot behavior
- `tests/bridge_json.rs` for camelCase and bridge draw-command JSON assertions
- `tests/local_package_smoke.rs` for the optional local `mashikaku` package smoke assertion

Replicate the current sample-package helper and assertions exactly as needed in each file. Do not broaden coverage.

- [ ] **Step 2: Remove the large in-file `#[cfg(test)] mod tests` block from `src/lib.rs`**

After the integration tests exist, delete the old monolithic web test module from `crates/iwm-runtime-web/src/lib.rs`.

- [ ] **Step 3: Run the web crate tests again**

Run:

```powershell
cargo test -p iwm-runtime-web
```

Expected:

```text
test result: ok
```

- [ ] **Step 4: Commit the web test split**

```bash
git add crates/iwm-runtime-web/tests crates/iwm-runtime-web/src/lib.rs
git commit -m "refactor(runtime-web): split bridge tests by responsibility"
```

### Task 5: Split `iwm-runtime-core` Types And Top-Level Orchestration

**Files:**
- Modify: `crates/iwm-runtime-core/src/lib.rs`
- Create: `crates/iwm-runtime-core/src/types.rs`
- Create: `crates/iwm-runtime-core/src/core.rs`
- Create: `crates/iwm-runtime-core/src/helpers.rs`

- [ ] **Step 1: Replace the core crate root with internal module declarations and stable re-exports**

Write `crates/iwm-runtime-core/src/lib.rs` as:

```rust
mod core;
mod diagnostics;
mod helpers;
mod logic;
mod movement;
mod render;
mod room_builder;
mod room_transitions;
mod types;

pub use core::RuntimeCore;
pub use types::{
    LoweredLogicEntry, LoweredLogicFile, LoweredLogicStatement, RuntimeCoreError, RuntimeInstance,
    RuntimePackage, RuntimePlayerSnapshot, RuntimeRoomState, RuntimeSnapshot, RuntimeStatus,
    RuntimeValue,
};
```

- [ ] **Step 2: Extract shared runtime-core types into `types.rs`**

Move the current definitions for:

- `LoweredLogicFile`
- `LoweredLogicEntry`
- `LoweredLogicStatement`
- `RuntimeValue`
- `RuntimePackage`
- `RuntimeStatus`
- `RuntimeInstance`
- `RuntimeRoomState`
- `RuntimePlayerSnapshot`
- `RuntimeSnapshot`
- `RuntimeCoreError`
- `impl From<RuntimeHostError> for RuntimeCoreError`

Use the current serde attributes and field names unchanged.

- [ ] **Step 3: Create `helpers.rs` for small pure helpers that are shared across later extracted modules**

Move only the current helper functions that do not define a stronger domain boundary, such as:

- player-instance identification helpers
- preferred player-name filtering
- numeric conversion helpers

Do not move room building, movement, logic, render, or diagnostics behavior into `helpers.rs`.

- [ ] **Step 4: Create `core.rs` and move the top-level `RuntimeCore` struct and public methods there**

Move:

- the `RuntimeCore` struct
- `load`
- `status`
- `tick_count`
- `current_room`
- `diagnostics`
- `snapshot`
- `request_room_transition`
- `render`
- `tick`
- `reload_room`

The method bodies should continue to call internal methods exactly as before; only module paths should change.

- [ ] **Step 5: Run the core crate tests to catch extraction regressions early**

Run:

```powershell
cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

### Task 6: Split `iwm-runtime-core` Room Building, Movement, Logic, Render, And Diagnostics

**Files:**
- Create: `crates/iwm-runtime-core/src/room_builder.rs`
- Create: `crates/iwm-runtime-core/src/room_transitions.rs`
- Create: `crates/iwm-runtime-core/src/movement.rs`
- Create: `crates/iwm-runtime-core/src/logic.rs`
- Create: `crates/iwm-runtime-core/src/render.rs`
- Create: `crates/iwm-runtime-core/src/diagnostics.rs`

- [ ] **Step 1: Extract room creation and room boot logic into `room_builder.rs`**

Move the current implementations for:

- `boot_default_room`
- `build_room`
- `sprite_metrics`

Also move any room-construction-local helpers needed for:

- spawn-point selection
- fallback player creation
- room instance population

- [ ] **Step 2: Extract room reset and transition handling into `room_transitions.rs`**

Move the current implementations for:

- `apply_pending_room_change`
- `reset_player_to_spawn`

Preserve the current ready-state behavior after reset and room transitions.

- [ ] **Step 3: Extract player movement and collision behavior into `movement.rs`**

Move the current implementations for:

- `step_player`
- movement constants such as `RUN_SPEED`, `JUMP_SPEED`, `GRAVITY`, `MAX_FALL_SPEED`
- axis stepping helpers
- collision helpers
- out-of-bounds detection

Preserve current solid/hazard semantics exactly.

- [ ] **Step 4: Extract lowered-logic execution into `logic.rs`**

Move the current implementations for:

- create-time lowered logic application
- step-time lowered logic execution
- assignment handling
- current narrow function-call handling such as `room_goto` and `game_restart`
- any small parsing helpers dedicated to lowered logic

Keep the currently supported lowered-logic subset unchanged.

- [ ] **Step 5: Extract frame building into `render.rs`**

Move the current implementations for:

- `build_render_frame`
- background/tile/sprite/fill command assembly

Preserve current draw command order and frame dimensions.

- [ ] **Step 6: Extract diagnostic recording into `diagnostics.rs`**

Move the current implementation for:

- `record_diagnostic`

Keep current diagnostic codes and message formatting unchanged.

- [ ] **Step 7: Run the core crate tests**

Run:

```powershell
cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

- [ ] **Step 8: Commit the core modularization**

```bash
git add crates/iwm-runtime-core/src
git commit -m "refactor(runtime-core): split runtime internals by responsibility"
```

### Task 7: Reorganize `iwm-runtime-core` Tests By Behavior Slice

**Files:**
- Create: `crates/iwm-runtime-core/tests/boot.rs`
- Create: `crates/iwm-runtime-core/tests/movement.rs`
- Create: `crates/iwm-runtime-core/tests/logic.rs`
- Create: `crates/iwm-runtime-core/tests/render.rs`
- Create: `crates/iwm-runtime-core/tests/transitions.rs`
- Modify: `crates/iwm-runtime-core/src/lib.rs`

- [ ] **Step 1: Create focused integration tests for boot and room construction behavior**

Move these assertions into `tests/boot.rs`:

- default room load
- missing room error
- fallback player spawn
- player start marker handling

Replicate the existing sample package helper and assertions exactly as needed.

- [ ] **Step 2: Create focused integration tests for movement behavior**

Move these assertions into `tests/movement.rs`:

- left/right movement
- jump behavior
- solid collision stop
- hazard reset signal

Replicate the existing sample package helper and assertions exactly as needed.

- [ ] **Step 3: Create focused integration tests for lowered logic behavior**

Move these assertions into `tests/logic.rs`:

- lowered create assignments
- lowered room creation assignments
- lowered step assignments
- lowered `room_goto`
- lowered `game_restart`

Replicate the existing sample package helper and assertions exactly as needed.

- [ ] **Step 4: Create focused integration tests for rendering and transitions**

Move these assertions into:

- `tests/render.rs` for frame command generation
- `tests/transitions.rs` for room transition, player reset, idle diagnostics, host diagnostics sink, and tick/frame submission behavior

Replicate the existing sample package helper and assertions exactly as needed.

- [ ] **Step 5: Remove the monolithic `#[cfg(test)]` block from the core source modules**

After the focused integration tests exist, delete the old in-file test block from the core crate source.

- [ ] **Step 6: Run the core crate tests again**

Run:

```powershell
cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

- [ ] **Step 7: Commit the core test split**

```bash
git add crates/iwm-runtime-core/tests crates/iwm-runtime-core/src
git commit -m "refactor(runtime-core): split runtime tests by behavior"
```

### Task 8: Document The New Runtime Crate Layout

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add a short runtime crate layout note to the README**

Add a concise note under the runtime-related repository contents or current phase section stating that:

- `iwm-runtime-host` now separates host contracts from default implementations
- `iwm-runtime-web` separates bridge models, translation, host wrapper, result storage, and FFI exports
- `iwm-runtime-core` separates runtime types, orchestration, room building, movement, logic, rendering, and diagnostics

Keep the note descriptive only. Do not change project phase claims.

- [ ] **Step 2: Commit the README update**

```bash
git add README.md
git commit -m "docs: describe modular runtime crate layout"
```

### Task 9: Final Verification For Runtime Modularization

**Files:**
- Modify: none

- [ ] **Step 1: Run formatting across the workspace**

Run:

```powershell
cargo fmt --all
```

Expected:

```text
no output
```

- [ ] **Step 2: Run all Rust tests**

Run:

```powershell
cargo test
```

Expected:

```text
test result: ok
```

- [ ] **Step 3: Run the runtime frontend tests**

Run:

```powershell
npm --prefix runtime test
```

Expected:

```text
all tests pass
```

- [ ] **Step 4: Optionally run browser smoke if the local environment is already prepared**

Run:

```powershell
npm --prefix runtime run test:browser
```

Expected:

```text
browser smoke passes
```

If the local browser smoke setup is missing packages, fixtures, or Playwright prerequisites, record that rather than changing scope.

- [ ] **Step 5: Commit final verification if needed**

```bash
git add -A
git commit -m "test: verify runtime rust modularization"
```

## Self-Review

Spec coverage for this plan:

- runtime-host modular split: covered
- runtime-web bridge/ffi/result-store split: covered
- runtime-core responsibility split: covered
- test reorganization by responsibility: covered
- API/behavior preservation: covered through narrow-first verification
- parser modularization: intentionally not covered
- runtime behavior changes: intentionally not covered

Placeholder scan:

- no `TODO`
- no `TBD`
- no “implement appropriately” placeholders
- every code-changing task names exact files and concrete responsibilities

Type consistency notes:

- current public exports remain re-exported from each `lib.rs`
- FFI entrypoint names remain `iwm_alloc`, `iwm_free`, `iwm_last_result_len`, `iwm_boot_json`, `iwm_set_input_json`, `iwm_tick`, `iwm_reset`, `iwm_select_room`, `iwm_snapshot_json`, `iwm_frame_json`, `iwm_diagnostics_json`
- `WebRuntimeHost` remains the bridge host wrapper type
- `RuntimeCore` remains the top-level runtime-core type
- `HeadlessHost` remains the composed default host type
