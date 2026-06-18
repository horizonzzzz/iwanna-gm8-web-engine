# Codebase Cleanup Batches Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Continue the multi-batch cleanup that reduces test duplication, splits oversized implementation files, and makes the repository easier for agents to read and change.

**Architecture:** Keep behavior stable while improving the test and module structure around existing crate boundaries. First classify and consolidate tests, then split large implementation files only after the tests are easier to reason about.

**Tech Stack:** Rust workspace, Cargo tests, Vite/Vitest runtime shell, Playwright smoke tests when local prerequisites exist, graphify for repository graph updates.

---

## Current State

Batch 0 and the first pass of Batch 1 are complete in the current branch:

- `docs/notes/testing-strategy.md` defines the repository test layers.
- `crates/iwm-runtime-core/src/tests/support.rs` now centralizes repeated player event-block construction.
- `crates/iwm-runtime-web/tests/support/mod.rs` now uses named fixture constructors instead of one large package literal.
- `runtime/src/test/packageFixtures.ts` provides a shared frontend runtime package fixture.

Current verification baseline:

```powershell
cargo test
npm --prefix runtime test
graphify update .
```

## Batch 1 Remainder: Test Support Builders

**Files:**
- Modify: `crates/iwm-runtime-core/src/tests/support.rs`
- Modify: `crates/iwm-runtime-core/src/tests/logic/*.rs`
- Modify: `crates/iwm-runtime-core/src/tests/movement.rs`
- Modify: `runtime/src/test/packageFixtures.ts`
- Modify as needed: frontend tests that construct partial runtime packages

- [ ] **Step 1: Split runtime-core support into small fixture constructors**

Extract focused helpers inside `support.rs` before moving tests:

```rust
fn runtime_manifest() -> RuntimeManifest
fn primary_room() -> RoomDefinition
fn secondary_room() -> RoomDefinition
fn player_object() -> ObjectDefinition
fn marker_object() -> ObjectDefinition
fn block_object() -> ObjectDefinition
fn checkpoint_object() -> ObjectDefinition
fn sparse_sprite_object() -> ObjectDefinition
fn sample_resources() -> ResourceIndex
```

Keep `sample_package()` as the public test entrypoint so existing tests remain readable.

- [ ] **Step 2: Add assertion helpers for common runtime test checks**

Add helper functions for repeated room/player/diagnostic access:

```rust
fn player(core: &RuntimeCore) -> &RuntimeInstance
fn player_mut(core: &mut RuntimeCore) -> &mut RuntimeInstance
fn player_var(core: &RuntimeCore, name: &str) -> Option<&RuntimeValue>
fn assert_no_runtime_blockers(core: &RuntimeCore)
```

Use them only where they remove repeated setup or repeated diagnostic filtering.

- [ ] **Step 3: Convert high-duplication tests opportunistically**

Start with files that call `sample_package()` and `host()` repeatedly:

```powershell
cargo test -p iwm-runtime-core logic::expressions
cargo test -p iwm-runtime-core logic::instances
cargo test -p iwm-runtime-core movement
```

Expected result: all selected tests pass with no behavior changes.

- [ ] **Step 4: Extend frontend fixtures only where full package literals repeat**

Keep `loadPackage.test.ts` fetch fixtures local because that test verifies file-boundary loading. Use `makeRuntimePackage()` for component, hook, session, and bridge tests that need an already-loaded package object.

Run:

```powershell
npm --prefix runtime test
```

Expected result: 42 frontend tests pass.

## Batch 2: Parser Test Split

**Files:**
- Split from: `crates/iwm-parser/tests/build_package_smoke.rs`
- Create: `crates/iwm-parser/tests/package_contract.rs`
- Create: `crates/iwm-parser/tests/resource_export_contract.rs`
- Create: `crates/iwm-parser/tests/logic_lowering_contract.rs`
- Create: `crates/iwm-parser/tests/event_tag_contract.rs`
- Keep: `crates/iwm-parser/tests/gml_lowering_precedence.rs`

- [ ] **Step 1: Move manifest and package output tests**

Move package output shape tests into `package_contract.rs`:

```rust
runtime_manifest_serializes_expected_fields
runtime_package_uses_ir_and_resource_index_outputs
build_package_writes_runtime_outputs_for_single_exe_input
logic_block_ids_use_stable_prefixes
logic_block_distinguishes_executable_vs_source_only
analysis_warnings_use_actionable_categories
event_block_ids_are_stable_and_parseable
room_transition_block_ids_follow_naming_convention
```

Run:

```powershell
cargo test -p iwm-parser --test package_contract
```

Expected result: moved tests pass.

- [ ] **Step 2: Move resource export tests**

Move sprite/background/audio/resource-index tests into `resource_export_contract.rs`.
Keep `game_assets_with_sprite_frame()` local to that file unless another parser test needs it.

Run:

```powershell
cargo test -p iwm-parser --test resource_export_contract
```

Expected result: moved tests pass.

- [ ] **Step 3: Move lowered logic tests**

Move lowered GML and raw logic tests into `logic_lowering_contract.rs`.
Convert repeated "input source -> expected lowered structure" cases to table-driven helpers when the test body only differs by source string and expected expression kind.

Run:

```powershell
cargo test -p iwm-parser --test logic_lowering_contract
```

Expected result: moved tests pass.

- [ ] **Step 4: Move event tag tests**

Move event-tag normalization tests into `event_tag_contract.rs`.
Convert the all-supported-events check to a compact table with event type, sub-event, and expected tag.

Run:

```powershell
cargo test -p iwm-parser --test event_tag_contract
```

Expected result: moved tests pass.

- [ ] **Step 5: Run parser verification**

Run:

```powershell
cargo test -p iwm-parser
```

Expected result: parser tests pass with the same coverage split across smaller files.

## Batch 3: Runtime-Core Test Consolidation

**Files:**
- Modify: `crates/iwm-runtime-core/src/tests/logic/expressions.rs`
- Modify: `crates/iwm-runtime-core/src/tests/logic/instances.rs`
- Modify: `crates/iwm-runtime-core/src/tests/logic/step.rs`
- Modify: `crates/iwm-runtime-core/src/tests/movement.rs`
- Modify: `crates/iwm-runtime-core/src/tests/logic/real_sample.rs`

- [ ] **Step 1: Convert expression cases into table-driven tests**

Group cases by runtime behavior:

- unary and binary expression evaluation
- GM numeric truthiness and comparisons
- object-name and instance-id helper resolution
- unsupported expression diagnostics

Run:

```powershell
cargo test -p iwm-runtime-core logic::expressions
```

Expected result: expression tests pass and the file has fewer repeated package setup blocks.

- [ ] **Step 2: Consolidate instance-create and destroy scenarios**

Use shared setup helpers for `instance_create()`, `instance_destroy()`, `with`, and post-create member writes.

Run:

```powershell
cargo test -p iwm-runtime-core logic::instances
```

Expected result: instance tests pass with scenario names still describing visible behavior.

- [ ] **Step 3: Separate pure movement math from room-runtime movement**

Keep pure helper tests near movement helpers when they do not need `RuntimeCore::load()`.
Leave room-runtime movement scenarios in `movement.rs`.

Run:

```powershell
cargo test -p iwm-runtime-core movement
```

Expected result: movement tests pass and test intent is easier to scan.

- [ ] **Step 4: Isolate local sample smoke tests**

Keep tests that depend on `runtime/public/packages/sample/` in `real_sample.rs`, but make every one skip cleanly when the local package is absent. Do not add new local-sample-only assertions to default contract tests.

Run:

```powershell
cargo test -p iwm-runtime-core logic::real_sample
```

Expected result: tests pass when sample exists and skip cleanly otherwise.

## Batch 4: Runtime-Core Implementation Split

**Files:**
- Split from: `crates/iwm-runtime-core/src/logic/statement.rs`
- Split from: `crates/iwm-runtime-core/src/logic/eval.rs`
- Split from: `crates/iwm-runtime-core/src/logic/mod.rs`
- Modify: `crates/iwm-runtime-core/src/logic.rs` or `crates/iwm-runtime-core/src/logic/mod.rs`

- [ ] **Step 1: Split statement execution by responsibility**

Create focused modules under `crates/iwm-runtime-core/src/logic/`:

```text
assignment.rs
calls.rs
control_flow.rs
diagnostics.rs
instances.rs
```

Keep public crate-local entrypoints stable while moving internals.

Run:

```powershell
cargo test -p iwm-runtime-core
```

Expected result: runtime-core tests pass after each module split.

- [ ] **Step 2: Split expression evaluation by responsibility**

Create focused modules under `crates/iwm-runtime-core/src/logic/`:

```text
eval_values.rs
eval_functions.rs
eval_variables.rs
```

Keep `evaluate_expr()` as the single callsite-facing function until consumers are ready for finer APIs.

Run:

```powershell
cargo test -p iwm-runtime-core
```

Expected result: runtime-core tests pass after each module split.

- [ ] **Step 3: Re-check docs when semantics move**

If behavior changes or runtime blockers change, update:

```text
docs/notes/runtime-wasm-gap-analysis.md
docs/notes/package-format-v1-runtime.md
```

No docs update is needed for pure file moves with no behavior change.

## Batch 5: Frontend Runtime Shell Test Cleanup

**Files:**
- Modify: `runtime/src/test/packageFixtures.ts`
- Modify: `runtime/src/runtime/wasmBridge.test.ts`
- Modify: `runtime/src/runtime/wasmSession.test.ts`
- Modify: `runtime/src/render/*.test.ts`
- Modify: `runtime/src/ui/hooks/useRuntimeShell.test.tsx`

- [ ] **Step 1: Add narrow frontend fixture builders**

Extend `packageFixtures.ts` with helpers only when repeated by at least two tests:

```ts
makeRoomDefinition()
makeResourceIndex()
makeWasmSnapshot()
makeWasmFrame()
```

Keep fetch-boundary fixtures inside `loadPackage.test.ts`.

- [ ] **Step 2: Keep bridge tests as contract tests**

Do not move low-level WASM memory tests behind broad UI helpers. Those tests protect bridge ABI behavior and should remain close to `wasmBridge.test.ts`.

Run:

```powershell
npm --prefix runtime test -- src/runtime/wasmBridge.test.ts
```

Expected result: bridge tests pass.

- [ ] **Step 3: Keep renderer tests command-focused**

Renderer tests should assert draw-command behavior and resource-cache interaction, not full package loading.

Run:

```powershell
npm --prefix runtime test -- src/render
```

Expected result: renderer tests pass.

- [ ] **Step 4: Run frontend verification**

Run:

```powershell
npm --prefix runtime test
npm --prefix runtime run build
```

Expected result: test and build pass.

## Batch 6: Agent Readability Pass

**Files:**
- Create or modify: `docs/notes/runtime-core-map.md`
- Modify: module-level comments in large runtime-core files after splits
- Modify: `docs/notes/testing-strategy.md` when test rules change

- [ ] **Step 1: Document runtime-core module map**

Create `docs/notes/runtime-core-map.md` with:

- module responsibility list
- runtime tick data flow
- logic execution module map
- diagnostics and host-boundary entrypoints

- [ ] **Step 2: Add short module comments where file names are not enough**

Use comments only to explain responsibility or non-obvious design constraints.
Do not add comments that restate code.

- [ ] **Step 3: Update graphify after code movement**

Run:

```powershell
graphify update .
```

Expected result: graphify reports a rebuilt code graph.

## Final Verification For Each Batch

Use the narrowest relevant test first, then broaden:

```powershell
cargo test -p <crate>
npm --prefix runtime test
cargo test
npm --prefix runtime run build
graphify update .
```

Only run browser smoke when local prerequisites are available:

```powershell
npm --prefix runtime run test:browser
```

Do not commit generated local sample packages or copyrighted sample binaries.
