# Parser And Runtime Next Steps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tighten the GM8 parser/runtime boundary, then push the WASM-first runtime toward sample-driven playability through explicit host traits, clearer package contracts, and a smaller set of proven runtime semantics.

**Architecture:** Keep parser work in Rust and keep runtime execution in the WASM-first core path. The parser owns GM8 extraction, raw and lowered logic preservation, and normalized package emission. The runtime side owns deterministic boot, input, movement, collision, room changes, and diagnostics through the narrow host traits in `iwm-runtime-host`, with the browser shell staying a consumer and fallback viewer rather than a second gameplay engine.

**Tech Stack:** Rust 1.77+, Cargo workspace, `serde`, `serde_json`, `anyhow`, `sha2`, `zip`, vendored `OpenGMK`, current `iwm-runtime-model` / `iwm-runtime-host` / `iwm-runtime-core` / `iwm-runtime-web`, Vite, TypeScript, Vitest, Playwright

---

## File Structure

Planned files for this phase:

- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-gold-sample.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`
- Modify: `docs/notes/opengmk-host-coupling-audit.md`
- Modify: `crates/iwm-parser/src/models.rs`
- Modify: `crates/iwm-parser/src/logic_export.rs`
- Modify: `crates/iwm-parser/src/gml_lowering.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `crates/iwm-runtime-model/src/lib.rs`
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Modify: `crates/iwm-runtime-core/src/lib.rs`
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/types.ts`
- Modify: `runtime/src/loadPackage.ts`
- Modify: `runtime/src/loadPackage.test.ts`
- Modify: `runtime/src/runtime/logicRunner.ts`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Modify: `runtime/src/runtime/wasmSession.ts`
- Modify: `runtime/src/runtime/wasmSession.test.ts`
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`

Responsibilities:

- `iwm-parser/src/models.rs`: runtime package contract and JSON shapes
- `iwm-parser/src/logic_export.rs`: event normalization and logic block classification
- `iwm-parser/src/gml_lowering.rs`: raw GML lowering into a narrow executable representation
- `iwm-parser/src/package_builder.rs`: package emission, analysis, and warning generation
- `iwm-runtime-model`: shared runtime package schema
- `iwm-runtime-host`: deterministic host traits and headless helpers
- `iwm-runtime-core`: boot, tick, room, collision, and player semantics
- `iwm-runtime-web`: WASM bridge for browser shell integration
- `runtime/src/runtime/*`: browser-side bridge/session/state glue only
- `runtime/src/ui/shell.ts`: shell controls, diagnostics, and fallback presentation

## Preconditions

Before starting this phase:

- detector work is already present and stable enough to classify inputs
- the parser workspace already exists and emits V1 runtime packages
- the current browser shell can load normalized packages and boot the WASM bridge
- the OpenGMK submodule under `vendor/OpenGMK/` must remain the runtime semantics reference
- the current `runtime/` app remains a shell and diagnostics harness, not the long-term gameplay engine

## Task 1: Lock The Parser Contract Around What The Runtime Actually Uses

**Files:**
- Modify: `crates/iwm-parser/src/models.rs`
- Modify: `crates/iwm-parser/src/logic_export.rs`
- Modify: `crates/iwm-parser/src/gml_lowering.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `docs/notes/package-format-v1-runtime.md`

- [ ] **Step 1: Identify the runtime-facing fields that are already stable and the ones that are still only hints**

Focus on fields that the WASM core and shell truly consume:

- `default_room_id`
- room and object event tags
- `transition_targets`
- `is_player`, `is_hazard`, `is_checkpoint`
- `scripts.ir.json` support classification
- raw and lowered logic outputs

- [ ] **Step 2: Add or adjust tests to pin the current package contract**

Use `crates/iwm-parser/tests/build_package_smoke.rs` to assert that:

- `manifest.json` includes the runtime-facing room and object counts
- `scripts.ir.json` still contains `action-list` and `source-only` blocks
- `logic.raw.json` and `logic.lowered.json` stay present
- `resources/index.json` remains the browser asset index

- [ ] **Step 3: Improve the parser warning model**

Make the warnings more actionable by separating:

- source lowering gaps
- unsupported event tags
- unsupported action families
- resource export limitations
- room transition uncertainty

- [ ] **Step 4: Keep lowering intentionally narrow**

Only extend `gml_lowering.rs` when a sample proves the runtime needs that syntax on the critical path. Do not expand the lowering surface just because a snippet is parseable.

- [ ] **Step 5: Re-run parser verification**

Run:

```bash
rtk cargo test -p iwm-parser
```

Expected:

```text
test result: ok
```

## Task 2: Make The Runtime Contract Explicit In Shared Types

**Files:**
- Modify: `crates/iwm-runtime-model/src/lib.rs`
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Modify: `runtime/src/types.ts`
- Modify: `runtime/src/loadPackage.ts`

- [ ] **Step 1: Add only the missing shared fields that remove browser-side guesswork**

Prefer additive fields only. Likely candidates are:

- room boot hints
- runtime-facing event tags
- explicit block support classifications
- diagnostics that explain which layer failed

- [ ] **Step 2: Keep Rust and TypeScript models aligned**

Update the shared schema and the browser loader together so the shell does not drift from the runtime package shape.

- [ ] **Step 3: Preserve fallback behavior**

Do not remove static room loading or diagnostics rendering while the runtime contract is still being tightened.

- [ ] **Step 4: Verify the loader still reads V1 packages**

Run:

```bash
rtk npm --prefix runtime test
```

Expected:

```text
pass
```

## Task 3: Narrow The Runtime Core To Proven Sample Semantics

**Files:**
- Modify: `crates/iwm-runtime-core/src/lib.rs`
- Modify: `crates/iwm-runtime-host/src/lib.rs`

- [ ] **Step 1: Write or extend tests for the critical gameplay path**

Use headless runtime tests to pin:

- default room boot
- player spawn selection
- left/right movement
- jump initiation
- solid collision
- hazard contact
- reset
- room transition

- [ ] **Step 2: Keep host boundaries narrow and explicit**

`RuntimeTimeHost`, `RuntimeInputHost`, `RuntimeRenderHost`, `RuntimeAudioHost`, `RuntimeFileHost`, `RuntimeExternalHost`, and `RuntimeDiagnosticsHost` should stay small and deterministic.

- [ ] **Step 3: Refine the first collision and movement behavior only as sample evidence requires**

Do not chase full GM8 physics. The first target is reliable platformer behavior on the gold sample.

- [ ] **Step 4: Keep unsupported behavior loud**

When externals, audio, or unsupported logic appear, record diagnostics instead of silently continuing.

- [ ] **Step 5: Re-run runtime core tests**

Run:

```bash
rtk cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

## Task 4: Thread Runtime Core Behavior Through The WASM Bridge

**Files:**
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/runtime/logicRunner.ts`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Modify: `runtime/src/runtime/wasmSession.ts`
- Modify: `runtime/src/runtime/wasmSession.test.ts`
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`

- [ ] **Step 1: Keep the bridge focused on snapshot exchange**

The bridge should keep exposing:

- boot
- input
- tick
- reset
- room selection
- diagnostics
- frame snapshots

- [ ] **Step 2: Make browser input transitions deterministic**

Ensure the session layer emits `jumpPressed` / `jumpReleased` edges and that restart behavior is explicit.

- [ ] **Step 3: Keep the shell usable when gameplay still fails**

If runtime boot or tick fails, the shell should still surface the package, diagnostics, and fallback room state.

- [ ] **Step 4: Keep transitional TypeScript helpers non-authoritative**

If `runtime/src/runtime/logicRunner.ts` remains in the repo, keep it limited to diagnostics or fallback intent parsing. Do not expand it into a second gameplay engine beside the WASM path.

- [ ] **Step 5: Re-run the runtime bridge tests**

Run:

```bash
rtk cargo test -p iwm-runtime-web
rtk npm --prefix runtime test
```

Expected:

```text
test result: ok
```

## Task 5: Update The Sample-Driven Runtime Notes

**Files:**
- Modify: `docs/notes/runtime-gold-sample.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`
- Modify: `docs/notes/opengmk-host-coupling-audit.md`
- Modify: `README.md`

- [ ] **Step 1: Mark the next highest blocker by layer**

For the current gold sample, separate blockers into:

- parser missing data
- runtime-core semantic gap
- wasm/web host gap
- shell-only issue

- [ ] **Step 2: Keep `IWBT_Dife` as the primary validation target**

Use secondary packages like `kamilia` and `mashikaku` only as smoke references unless they prove a more urgent blocker.

- [ ] **Step 3: Keep OpenGMK and GM8Decompiler roles distinct**

OpenGMK stays the runtime semantics reference. GM8Decompiler stays comparison-only.

- [ ] **Step 4: Update the README phase summary**

Make the runtime direction explicit:

- parser emits the runtime package
- runtime core executes the package
- shell is a viewer and control surface, not the gameplay engine

## Task 6: End-To-End Verification For The Current Milestone

**Files:**
- Modify: none

- [ ] **Step 1: Run Rust tests**

```bash
rtk cargo test
```

- [ ] **Step 2: Run frontend tests**

```bash
rtk npm --prefix runtime test
```

- [ ] **Step 3: Build the runtime shell**

```bash
rtk npm --prefix runtime run build
```

- [ ] **Step 4: Rebuild the WASM bridge**

```bash
rtk cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
```

- [ ] **Step 5: Sync the rebuilt WASM artifact into the browser shell**

```bash
rtk npm --prefix runtime run sync:wasm
```

- [ ] **Step 6: Smoke test the gold sample in the browser**

```bash
rtk npm --prefix runtime run dev -- --host 127.0.0.1
```

Verify that the shell can still load the package, boot the bridge, and report explicit diagnostics even when a sample still has missing runtime semantics.

## Self-Review

Spec coverage for this plan:

- parser contract tightening: covered
- shared runtime schema alignment: covered
- runtime-core semantic slicing: covered
- WASM bridge and shell threading: covered
- sample-driven blocker tracking: covered
- full GM8 coverage: intentionally deferred
- broad licensing decision: surfaced, not solved here

Placeholder scan:

- no TBD/TODO placeholders
- no "handle appropriately" style steps
- no undefined function names in later tasks

Type consistency notes:

- parser output remains `manifest.json`, `rooms.json`, `objects.json`, `scripts.ir.json`, `logic.raw.json`, `logic.lowered.json`, `analysis.json`, and `resources/index.json`
- runtime contract continues through `RuntimePackage`
- bridge entrypoints remain boot/input/tick/reset/select-room/snapshot/frame/diagnostics

Next planned document after this phase:

- a follow-up runtime-hardening or parser-contract plan, depending on the next blocker the sample corpus proves
