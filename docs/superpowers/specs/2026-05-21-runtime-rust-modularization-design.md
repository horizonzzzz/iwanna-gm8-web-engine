# Runtime Rust Modularization Design

## Overview

This document defines a focused modularization pass for the heaviest Rust runtime crates in the repository.

The goal is to reduce file-level responsibility overload without changing runtime behavior, public APIs, or test semantics. The immediate target is to make the runtime codebase easier for future agents and human contributors to navigate, modify, and verify in smaller units.

This is an internal structure change, not a runtime feature change.

## Scope

### In Scope

- `crates/iwm-runtime-core/`
- `crates/iwm-runtime-web/`
- `crates/iwm-runtime-host/`
- Internal module extraction inside those crates
- Re-export organization needed to preserve current public crate surfaces
- Test reorganization for those runtime crates when it helps align tests to the new module responsibilities

### Out Of Scope

- Parser crate modularization
- Detector crate modularization
- Behavior changes in runtime execution
- Public API redesign
- FFI shape changes
- Package format changes
- Renaming externally consumed types or functions
- Opportunistic cleanup unrelated to modular boundaries

## Problem Statement

The runtime crates have accumulated several very large Rust source files, especially:

- `crates/iwm-runtime-core/src/lib.rs`
- `crates/iwm-runtime-web/src/lib.rs`
- `crates/iwm-runtime-host/src/lib.rs`

These files currently mix multiple responsibilities in a single compilation unit, including:

- shared data types
- runtime orchestration
- movement and collision logic
- rendering-frame construction
- lowered-logic execution
- host contracts and default host implementations
- browser bridge models
- FFI result-buffer management
- exported WASM entrypoints
- large embedded test sections

This creates three concrete costs:

1. contributors must load too much context to make a small change
2. unrelated edits are more likely to conflict in the same file
3. future agents are forced to reason across many responsibilities at once

The refactor should reduce those costs without creating architectural churn.

## Goals

- Split runtime code by responsibility, not by arbitrary line count
- Keep the current public API and behavior stable
- Make top-level crate entry files small and navigational
- Reduce accidental coupling between unrelated runtime concerns
- Make it practical for future agents to work within one focused module at a time

## Non-Goals

- Perfectly even module sizes
- Extracting every helper into its own file
- Introducing new abstraction layers just to make the tree look cleaner
- Solving runtime semantic gaps as part of this effort
- Refactoring parser/runtime boundaries

## Design Principles

### 1. Preserve behavior first

This modularization pass is explicitly structure-preserving. If a change risks altering runtime semantics, that change is outside scope.

### 2. Split only on real responsibility boundaries

A file should be split when it currently mixes multiple concerns that future work will likely touch independently. A cohesive chunk may remain intact even if it is not tiny.

### 3. Keep public surfaces stable

Existing crate users should continue to import the same exported types and functions after the split. Compatibility should be preserved with internal modules and `pub use`.

### 4. Keep top-level files orchestration-only

After modularization, each crate root should mainly declare modules and expose stable exports. It should not continue to hold most of the implementation.

### 5. Prefer directional dependencies

Modules should depend on lower-level shared types and traits, not on peers in circular ways. If a shared helper is needed, extract it deliberately instead of allowing peer-to-peer tangling.

## Recommended Approach

Use an internal, crate-by-crate modular split while preserving all current public exports.

This approach is preferred because it gives the project the structural benefits of modularization without silently turning the effort into a behavior refactor or public API cleanup. It also allows each crate to be verified independently after extraction.

Alternatives that were considered but rejected for this phase:

- broad all-runtime-and-parser modularization in one pass
  - rejected because the change surface is too wide for a purely structural refactor
- modularization plus naming/interface cleanup
  - rejected because it would blur the boundary between file extraction and behavioral refactoring
- test-only splitting
  - rejected because it would leave the main maintenance problem untouched

## Target Crates

### `iwm-runtime-host`

This crate should become the cleanest foundational layer of the runtime stack.

Current mixed responsibilities include:

- core runtime types
- host traits
- deterministic utilities
- default host implementations
- composed headless host
- crate-local tests

Recommended internal structure:

- `src/lib.rs`
  - module declarations
  - stable `pub use` exports
- `src/types.rs`
  - runtime buttons
  - colors
  - draw commands
  - diagnostics
  - external-call value types
  - host error types
- `src/traits.rs`
  - `RuntimeTimeHost`
  - `RuntimeInputHost`
  - `RuntimeRenderHost`
  - `RuntimeAudioHost`
  - `RuntimeFileHost`
  - `RuntimeExternalHost`
  - `RuntimeDiagnosticsHost`
  - `RuntimeHost`
- `src/clock.rs`
  - `DeterministicClock`
- `src/input.rs`
  - `SnapshotInputHost`
- `src/render.rs`
  - `NullRenderHost`
- `src/audio.rs`
  - `NoopAudioHost`
- `src/files.rs`
  - `MemoryFileHost`
- `src/externals.rs`
  - `RejectingExternalHost`
- `src/diagnostics.rs`
  - `VecDiagnosticsHost`
- `src/headless.rs`
  - `HeadlessHost`

Dependency rule:

- `types.rs` and `traits.rs` are the base layer
- implementation modules depend on those base modules
- `headless.rs` may compose all implementation modules
- implementation modules should not depend on `headless.rs`

### `iwm-runtime-web`

This crate should clearly separate browser bridge semantics from raw FFI exports.

Current mixed responsibilities include:

- bridge-facing JSON models
- host wrapper state
- snapshot/frame translation
- result buffer storage
- FFI exports
- tests

Recommended internal structure:

- `src/lib.rs`
  - module declarations
  - stable exports required by current consumers and tests
- `src/bridge_types.rs`
  - `WebInputState`
  - `BridgePlayerSnapshot`
  - `BridgeSnapshot`
  - `BridgeDrawCommand`
  - `BridgeFrameSnapshot`
- `src/web_runtime_host.rs`
  - `WebRuntimeHost`
  - boot/tick/reset/select-room/snapshot/diagnostics/frame methods
- `src/translate.rs`
  - conversion from runtime-core snapshots and draw commands into bridge JSON shapes
- `src/result_store.rs`
  - global result storage
  - UTF-8 payload reading
  - result serialization helpers
- `src/ffi.rs`
  - exported `#[no_mangle]` functions

Recommended test split:

- host behavior tests
- JSON/translation tests
- local package smoke tests

Dependency rule:

- `ffi.rs` should not contain core business logic
- `ffi.rs` calls into `web_runtime_host.rs` and `result_store.rs`
- translation logic stays centralized in `translate.rs`

### `iwm-runtime-core`

This is the heaviest runtime crate and should be treated as the last and most careful split.

Current mixed responsibilities include:

- package and lowered-logic data types
- runtime state types
- runtime orchestration
- room building
- player selection fallback
- movement and collision
- room reset and transition handling
- lowered-logic execution
- render-frame construction
- diagnostics helpers
- many tests in one file

Recommended internal structure:

- `src/lib.rs`
  - module declarations
  - stable `pub use` exports
- `src/types.rs`
  - `RuntimePackage`
  - lowered-logic types
  - runtime value/state types
  - snapshot types
  - error types
- `src/core.rs`
  - `RuntimeCore`
  - top-level public methods such as load/tick/render/reload/snapshot
- `src/room_builder.rs`
  - default room boot
  - room construction
  - sprite metrics
  - fallback player creation
  - spawn-point selection
- `src/room_transitions.rs`
  - queued room transition handling
  - room reset handling
  - reset-to-spawn helpers
- `src/movement.rs`
  - movement application
  - axis stepping
  - solid/hazard collision checks
  - out-of-bounds transition detection
- `src/logic.rs`
  - create-time lowered logic
  - step-time lowered logic
  - variable assignment support
  - narrow supported function dispatch such as `room_goto` and `game_restart`
- `src/render.rs`
  - render-frame building
  - draw-command assembly
- `src/diagnostics.rs`
  - diagnostic recording helpers
- `src/helpers.rs`
  - small shared pure helpers only when they do not justify a stronger domain module

Recommended test split:

- `tests/boot.rs`
- `tests/movement.rs`
- `tests/logic.rs`
- `tests/render.rs`
- `tests/transitions.rs`

If some tests must remain crate-internal because they require non-public internals, they can instead become focused `#[cfg(test)]` module files under `src/`.

Dependency rule:

- `core.rs` is the top-level orchestrator
- implementation modules should share state through `types.rs`
- peer modules should not accumulate circular dependencies
- shared helper extraction is allowed only when it prevents duplication without hiding responsibility

## Migration Strategy

The refactor should proceed crate-by-crate, not as one large cross-cutting rewrite.

Recommended order:

1. `iwm-runtime-host`
2. `iwm-runtime-web`
3. `iwm-runtime-core`

Reasoning:

- `iwm-runtime-host` has the most stable and infrastructure-like boundaries
- `iwm-runtime-web` benefits from a cleaner host layer and has clearer mechanical seams than `runtime-core`
- `iwm-runtime-core` has the highest semantic density and should be split after the surrounding runtime layers are already clarified

Within each crate, use this extraction pattern:

1. create target module files
2. move one responsibility group at a time
3. restore exports through `lib.rs`
4. run the narrowest relevant crate tests
5. continue only after the extracted crate is green

This keeps failures local and prevents a purely structural change from becoming difficult to debug.

## API Compatibility Requirements

The following must remain stable across the modularization:

- exported crate types currently consumed by sibling crates
- exported runtime functions and methods
- current FFI entrypoint names
- current browser-facing JSON field names
- current test entry behavior

Internal symbol relocation is allowed. External contract changes are not.

## Testing Strategy

Verification should remain behavior-preserving and narrow-first.

For each crate:

- run crate-local Rust tests immediately after extraction work in that crate
- only proceed to dependent crates when the current crate passes

For milestone verification after all three crates are split:

- run `cargo test`
- run `npm --prefix runtime test`

Optional but useful after the Rust split is complete:

- run `npm --prefix runtime run test:browser`
  - only if local browser-smoke prerequisites are already satisfied in the workspace

Success criteria:

- all existing tests still pass
- no consumer-facing API changes are required
- responsibilities are distributed across focused internal modules
- crate roots become navigational rather than implementation-heavy

## Risks

### 1. Hidden coupling inside `iwm-runtime-core`

Some helpers may currently rely on direct access to state in ways that become awkward after extraction. The mitigation is to extract by responsibility gradually and use shared type modules rather than inventing new abstractions.

### 2. Accidental public API drift

Moving items between modules can unintentionally change visibility or import paths. The mitigation is to preserve stable exports explicitly through `pub use` in `lib.rs`.

### 3. FFI breakage through mechanical movement

`iwm-runtime-web` export functions are easy to break if support utilities move carelessly. The mitigation is to isolate result-buffer and translation logic first, then keep `ffi.rs` thin.

### 4. Over-splitting

Too many tiny modules would make the code harder to follow rather than easier. The mitigation is to split only when the responsibility boundary is real and likely to matter for future maintenance.

## Expected Outcome

After this refactor:

- runtime contributors can open smaller, purpose-specific files
- future agents can work on one runtime concern without repeatedly loading unrelated logic
- merge conflicts should drop for common runtime work
- the project will have clearer internal runtime boundaries without paying the cost of a public API redesign

## Implementation Handoff Constraint

The implementation plan for this design should keep the effort strictly structural:

- no runtime behavior changes
- no parser changes
- no package contract changes
- no new runtime features

If implementation uncovers a genuine semantic bug, it should be documented separately rather than silently folded into the modularization work.
