# OpenGMK Runtime Host Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract a headless/browser-hostable OpenGMK-derived runtime core behind narrow project-owned host traits and prove a deterministic boot/tick smoke path through the browser-facing WASM bridge.

**Architecture:** OpenGMK `gm8emulator` remains the semantic reference for runtime behavior, but its desktop boot flow is not the target. This plan narrows the host boundary, keeps the current `runtime/` app as a shell and diagnostics surface, and proves the smallest boot/tick loop that can later be driven by a browser host. It does not try to complete GM8 parity or widen TS gameplay semantics.

**Tech Stack:** Rust workspace, vendored `OpenGMK`, `iwm-runtime-host`, `iwm-runtime-core`, `iwm-runtime-web`, WASM (`wasm32-unknown-unknown`), Vite/TypeScript shell, Vitest/Playwright.

---

## File Structure

Planned files for this phase:

- Modify: `docs/notes/opengmk-host-coupling-audit.md`
- Modify: `docs/notes/runtime-vendor-reference-map.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Modify: `crates/iwm-runtime-host/src/headless.rs`
- Modify: `crates/iwm-runtime-core/src/core.rs`
- Modify: `crates/iwm-runtime-core/src/logic.rs`
- Modify: `crates/iwm-runtime-core/src/tests/support.rs`
- Modify: `crates/iwm-runtime-core/src/tests/logic.rs`
- Modify: `crates/iwm-runtime-core/src/tests/lifecycle.rs`
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Modify: `runtime/src/runtime/wasmSession.ts`
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`
- Modify: `runtime/tests/browser/runtime-shell.spec.ts`

Responsibilities:

- `docs/notes/opengmk-host-coupling-audit.md`: host-boundary audit record
- `docs/notes/runtime-vendor-reference-map.md`: vendor reference map for runtime semantics
- `iwm-runtime-host`: the narrow host traits and headless implementations
- `iwm-runtime-core`: deterministic headless runtime orchestration and test harness
- `iwm-runtime-web`: WASM bridge surface for browser integration
- `runtime/`: browser shell, diagnostics, and bridge session glue only

## Preconditions

Before starting this phase:

- the parser contract plan is either merged or at least frozen at the current shared node set
- the workspace already passes `cargo test`
- the OpenGMK vendor subtree is present locally
- the runtime mainline remains the OpenGMK-derived WASM path, not a project-owned TS gameplay engine

## Task 1: Codify The OpenGMK Host Boundary

**Files:**
- Modify: `docs/notes/opengmk-host-coupling-audit.md`
- Modify: `docs/notes/runtime-vendor-reference-map.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`
- Modify: `crates/iwm-runtime-host/src/lib.rs`

- [ ] **Step 1: Update the audit note to reflect the current extraction target**

Make the audit note say explicitly:

- `gm8emulator` is the runtime-semantics source of truth
- the extraction target is host separation, not copying the desktop entrypoint
- windowing, audio, externals, and recording are host concerns, not core semantics

- [ ] **Step 2: Keep the reference map pointed at semantic files**

Keep the runtime reference map focused on:

- `movement.rs`
- `events.rs`
- `transition.rs`
- `input.rs`
- renderer files only as host-surface references

Do not expand the map into a second gameplay-engine spec.

- [ ] **Step 3: Make the host traits the only public runtime dependency surface**

`crates/iwm-runtime-host/src/lib.rs` should stay small and deterministic. The intent is to keep these surfaces explicit:

- `RuntimeTimeHost`
- `RuntimeInputHost`
- `RuntimeRenderHost`
- `RuntimeAudioHost`
- `RuntimeFileHost`
- `RuntimeExternalHost`
- `RuntimeDiagnosticsHost`

- [ ] **Step 4: Update the runtime gap note**

The gap note should say that the next blocker is host extraction and headless boot, not more TS gameplay semantics.

- [ ] **Step 5: Re-run a targeted workspace check**

Run:

```bash
rtk cargo test -p iwm-runtime-host
```

Expected:

```text
test result: ok
```

## Task 2: Build The Headless Runtime Smoke Path

**Files:**
- Modify: `crates/iwm-runtime-core/src/core.rs`
- Modify: `crates/iwm-runtime-core/src/logic.rs`
- Modify: `crates/iwm-runtime-core/src/tests/support.rs`
- Modify: `crates/iwm-runtime-core/src/tests/logic.rs`
- Modify: `crates/iwm-runtime-core/src/tests/lifecycle.rs`

- [ ] **Step 1: Add or extend internal tests around the headless host**

Use the existing `src/tests` layout so tests can inspect `pub(crate)` state without leaking a public test-only API. Pin these behaviors:

- default room boot
- deterministic tick progression
- input edges from `HeadlessHost`
- room transition
- reset
- diagnostics when an unsupported external is attempted

- [ ] **Step 2: Keep the current runtime-core as the harness**

Do not widen the runtime-core surface to mimic every OpenGMK subsystem. Use it as the deterministic smoke harness that validates host separation and the runtime contract while the extraction path is prepared.

- [ ] **Step 3: Ensure `HeadlessHost` remains the canonical no-op host**

`crates/iwm-runtime-host/src/headless.rs` should continue to compose the minimal hosts:

```rust
pub struct HeadlessHost {
    pub clock: DeterministicClock,
    pub input: SnapshotInputHost,
    pub renderer: NullRenderHost,
    pub audio: NoopAudioHost,
    pub files: MemoryFileHost,
    pub externals: RejectingExternalHost,
    pub diagnostics: VecDiagnosticsHost,
}
```

The plan is to keep this shape explicit and stable while the runtime semantics are extracted behind it.

- [ ] **Step 4: Run the runtime-core tests**

Run:

```bash
rtk cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

## Task 3: Keep The WASM Bridge Focused On Snapshot Exchange

**Files:**
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Modify: `runtime/src/runtime/wasmSession.ts`
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`
- Modify: `runtime/tests/browser/runtime-shell.spec.ts`

- [ ] **Step 1: Preserve the bridge API surface**

Keep the bridge centered on:

- boot
- snapshot
- frame
- set input
- tick
- reset
- room selection
- diagnostics

Do not grow a parallel gameplay engine in `runtime/src/runtime/logicRunner.ts`; keep that layer as fallback/diagnostic intent only.

- [ ] **Step 2: Keep the session edge handling deterministic**

`WasmRuntimeSession` should continue to derive `jumpPressed` and `jumpReleased` from input edges, then clear one-shot flags after each step. That keeps browser input deterministic and keeps gameplay semantics inside the runtime core, not in shell glue.

- [ ] **Step 3: Update the shell tests for the browser smoke path**

The browser shell tests should validate:

- the WASM bridge can be loaded
- boot returns a runtime snapshot
- the shell still falls back cleanly when the bridge is missing
- diagnostics remain visible even when runtime execution is incomplete

- [ ] **Step 4: Build and test the WASM module**

Run:

```bash
rtk cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
rtk npm --prefix runtime test
```

Expected:

```text
build succeeds
tests pass
```

## Task 4: Set The Cutover Rule For Parallel Execution

**Files:** none

- [ ] **Step 1: Adopt the gate between parser and runtime work**

Use this rule after the parser contract freeze:

- parser may continue only on shapes already justified by gold-sample evidence
- runtime may continue only on host extraction or headless smoke gaps
- contract changes require a checkpoint before runtime consumers absorb them

- [ ] **Step 2: Re-run workspace verification at the end**

Run:

```bash
rtk cargo test
rtk npm --prefix runtime run test:browser
```

Expected:

```text
test result: ok
browser smoke passes
```

## Self-Review

Coverage check:

- OpenGMK host boundary: covered
- headless runtime smoke path: covered
- browser WASM bridge focus: covered
- runtime notes sync: covered
- parser contract changes: intentionally out of scope for this plan
