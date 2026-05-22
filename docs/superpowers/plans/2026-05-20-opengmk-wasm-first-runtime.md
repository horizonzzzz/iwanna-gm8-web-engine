# OpenGMK WASM-First Runtime Implementation Plan

> **Implementation status note (2026-05-20):**
>
> - `crates/iwm-runtime-model/`, `crates/iwm-runtime-host/`, `crates/iwm-runtime-core/`, and `crates/iwm-runtime-web/` now exist
> - the browser shell can now load a frontend-facing WASM bridge after `cargo build -p iwm-runtime-web --target wasm32-unknown-unknown` and `npm --prefix runtime run sync:wasm`
> - the remaining work in this plan is to move from feasibility spike scaffolding toward fuller host fidelity, not to recreate these crates from scratch

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the previous TS-first gameplay-runtime direction with a WASM-first runtime path centered on adapting OpenGMK's `gm8emulator` into a browser-hosted execution core, while retaining the existing `runtime/` frontend as a shell, diagnostics surface, and package/debug harness.

> **Route decision update (2026-05-22):**
>
> - runtime mainline remains the OpenGMK-derived WASM path
> - parser mainline now explicitly includes replacing shallow GML token splitting with a real structured parser-owned contract for the IWanna-critical subset
> - the repository should no longer spend major effort extending a parallel TS gameplay runtime

**Architecture:** Keep Rust as the backend/parser language. Stop investing in the project-owned TypeScript gameplay runtime as the long-term execution engine. Instead:

1. keep `iwm-parser` producing runtime-oriented package outputs and diagnostics
2. upgrade parser-owned GML lowering toward real structured call/expression output for the IWanna-critical path
3. extract a host-agnostic execution core from OpenGMK `gm8emulator`
4. define host traits for render, input, audio, file/environment, and externals
5. implement a web host around a WASM-compiled runtime core
6. reuse the current `runtime/` app as the browser shell around the WASM core

**Important constraint note:** This plan is technically aligned with the desired end state, but it is subject to the existing OpenGMK license warning. `gm8emulator` is `GPL-2.0-only`; this is not a side note. The current repository direction assumes a GPL-2.0-compatible runtime path unless the architecture changes. Before productization or wider distribution, license validation for shipping a browser runtime derived from or linked against OpenGMK code is a required gate, not an optional follow-up.

**Important scope note:** The first milestone is not “full browser playability.” The first milestone is a feasibility spike proving that OpenGMK runtime logic can be separated from desktop host concerns and driven from a browser-compatible host boundary. Do not start by trying to port every subsystem at once.

**Tech Stack:** Rust 1.77+, Cargo workspace, vendored OpenGMK submodule, `gm8exe`, browser-hosted WASM target, current `runtime/` Vite shell, TypeScript only for host UI/glue

---

## Route Decision

This project is now explicitly **WASM-first** for runtime execution.

That means:

- the existing TypeScript runtime remains useful as a shell and diagnostic harness
- the existing TypeScript gameplay execution path is no longer the strategic engine direction
- do not continue expanding TS collision/movement/logic emulation as if it were the final compatibility layer
- future gameplay-fidelity work should accumulate in the OpenGMK-derived runtime core and browser host

### Why this direction was chosen

- the end goal requires runner-level fidelity, not heuristic approximation
- the current TS runtime already shows structural progress but not semantic parity
- continuing the TS gameplay path would duplicate work that must later be replaced by the WASM runtime
- OpenGMK already contains much deeper GM8 semantic knowledge than the project-owned TS runtime
- the remaining parser/runtime break is now understood as a contract problem, not only a host problem; without structured parser output, the runtime cannot execute semantics cleanly even when the host path exists

### What remains useful from the current frontend

- package loading
- resource inspection
- manifest / analysis display
- diagnostics UI
- local sample switching
- future runtime controls and overlays

### What should no longer be treated as the strategic engine

- TS-side movement rules
- TS-side collision semantics as final truth
- TS-side ad hoc source-snippet intent parsing
- TS-side action-list execution as the main compatibility path

---

## File Structure

Planned files for this phase:

- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-gold-sample.md`
- Modify: `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md`
- Create: `docs/superpowers/plans/2026-05-20-opengmk-wasm-first-runtime.md`
- Existing in repo: `crates/iwm-runtime-core/`
- Existing in repo: `crates/iwm-runtime-host/`
- Existing in repo: `crates/iwm-runtime-web/`

Responsibilities:

- `iwm-runtime-core`: host-agnostic runtime core extracted or adapted from OpenGMK execution logic
- `iwm-runtime-host`: trait definitions and host-facing contracts
- `iwm-runtime-web`: WASM adapter plus browser host entrypoints
- `runtime/`: browser shell, diagnostics panel, package picker, renderer glue, future devtools

---

## Preconditions

Before starting this phase:

- current parser output should still build and load in the frontend shell
- the OpenGMK submodule under `vendor/OpenGMK/` must be initialized
- the current repo should treat `gm8emulator` as a study and controlled-integration source, not an opaque black box
- the team must accept that browser runtime work is now blocked by both host-boundary extraction and parser-contract quality, not by more TS gameplay patching

---

## Phase 4A: Feasibility Spike

### Objective

Prove that OpenGMK runtime execution can be separated from desktop host concerns enough to support:

- loading parsed GM8 assets
- booting game state
- advancing at least one deterministic update tick
- reporting runtime diagnostics through a narrow API

This milestone does **not** require:

- browser rendering parity
- audio playback
- DLL support
- menu/UI parity
- a fully interactive sample

This milestone also does **not** justify keeping shallow parser lowering as the long-term plan. The spike should enumerate where host extraction is blocked by parser contract weakness versus genuine runner coupling.

---

## Parser Enabling Track

The runtime route depends on a parser-side upgrade.

Immediate parser goals:

- stop treating `gml_lowering.rs` string splitting as a viable long-term execution contract
- preserve structured function-call shape for the IWanna-critical subset
- preserve variable/member/index access structure needed for instance/global lookup
- keep unsupported syntax explicit instead of flattening it into misleading pseudo-structure
- in the next development cycle, make member access, index access, and binary expressions part of the minimum structured subset that reaches runtime consumers

The first target is not "all GML". The first target is "enough structured semantics that the runtime can distinguish `instance_create(x, y - 4, player2)` from an opaque string blob".

### Success Criteria

- a minimal Rust executable or test harness can instantiate the extracted runtime core without `ramen` window ownership
- the runtime can boot a known sample package or raw GM8 asset set without a desktop window loop
- one or more ticks can run in a deterministic test path
- core subsystems that still block headless/web execution are enumerated explicitly

---

## Task 1: Reframe Project Docs Around WASM-First

**Files:**
- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-gold-sample.md`
- Modify: `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md`

- [ ] **Step 1: Update the README current phase text**

Make it clear that:

- runtime direction is now WASM-first
- the current `runtime/` app is a shell and diagnostics harness
- TS gameplay execution is transitional, not the final compatibility engine

- [ ] **Step 2: Update the runtime package note**

Clarify that:

- the runtime package remains useful for shell inspection and future web-host input
- `scripts.ir.json` and current runtime hints should not be treated as the final execution contract if the WASM core consumes richer runtime data later
- current TS execution notes are transitional

- [ ] **Step 3: Update the gold sample note**

Reframe `IWBT_Dife` and `Kamilia` as:

- validation samples for runtime-core bring-up
- not proof that the TS runtime should be extended further as the final engine

- [ ] **Step 4: Mark the old minimal-playable-TS-runtime plan as superseded in direction**

Do not delete the document. Add a prominent note stating:

- it reflects the previous TS-first runtime direction
- it may still contain useful shell/UI/debug tasks
- gameplay-engine work should now follow the WASM-first plan instead

---

## Task 2: Audit OpenGMK Host Coupling

**Files:**
- Modify: none initially
- Output: audit notes in the new plan or follow-up note

- [ ] **Step 1: Identify the core host-bound subsystems in `gm8emulator`**

At minimum classify:

- window/event loop
- render backend
- input capture and key mapping
- audio output
- filesystem / temp dir / included-file export
- process spawning / shell access
- DLL / external function support
- recording/debug UI

- [ ] **Step 2: Separate “must keep for first browser boot” from “defer safely”**

Examples:

- must keep early: game state init, room loading, events, object lifecycle, collision, GML/action execution
- can defer: recording UI, clipboard, ffmpeg capture, broad DLL support, native process launch

- [ ] **Step 3: Produce an explicit blocker list**

For each blocker, record:

- source module
- why it is host-bound
- whether it can be stubbed, deferred, or abstracted behind a trait

---

## Task 3: Define Runtime Host Traits

**Files:**
- Existing: `crates/iwm-runtime-host/src/lib.rs`

- [ ] **Step 1: Define the minimal host interfaces**

Expected early host trait groups:

- `RuntimeTimeHost`
- `RuntimeInputHost`
- `RuntimeRenderHost`
- `RuntimeAudioHost`
- `RuntimeFileHost`
- `RuntimeExternalHost`
- `RuntimeDiagnosticsHost`

- [ ] **Step 2: Keep the first trait surface minimal**

Do not mirror every OpenGMK subsystem one-for-one.

The first host layer should support:

- deterministic tick timing
- key/button state reads
- presenting or collecting draw commands
- optional no-op audio
- controlled file access
- explicit unsupported-external behavior

- [ ] **Step 3: Decide the first rendering strategy**

Choose one of:

- immediate browser-facing draw command stream
- retained internal render state plus host-present step
- temporary headless/null render host for feasibility spike

For the first spike, a null/headless renderer is acceptable if it unblocks core execution extraction.

---

## Task 4: Extract Or Wrap A Headless Runtime Core

**Files:**
- Existing: `crates/iwm-runtime-core/`
- Modify later: vendored integration adapters as needed

- [ ] **Step 1: Prove that core initialization can run without desktop window ownership**

This may require:

- splitting launch logic
- deferring renderer creation
- replacing direct `ramen`/window setup with injected host services

- [ ] **Step 2: Build a headless smoke test**

Target:

- load known game assets
- construct runtime state
- enter the first room
- run a deterministic small number of ticks

- [ ] **Step 3: Stub or defer non-critical subsystems**

Allowed early stubs:

- no-op audio
- blocked DLL calls with explicit diagnostics
- no recording UI
- no clipboard
- no external process launching

- [ ] **Step 4: Record the exact reasons for any remaining boot failures**

Do not patch around failures opaquely. Keep a written list of:

- required GM8 functions
- render assumptions
- filesystem assumptions
- external dependency assumptions

---

## Task 5: Add A Browser WASM Host Prototype

**Files:**
- Existing: `crates/iwm-runtime-web/`
- Modify later: `runtime/src/`

- [ ] **Step 1: Compile the extracted core for a browser-compatible WASM target**

The first target is not polished UX. It is proof that:

- the core builds for WASM
- required host callbacks can be supplied from JS/TS

- [ ] **Step 2: Connect the current `runtime/` shell to the WASM host**

The shell should remain responsible for:

- selecting a package
- surfacing diagnostics
- offering pause/reset/dev controls
- possibly drawing debug overlays

- [ ] **Step 3: Start with a constrained host implementation**

First browser host may intentionally support only:

- keyboard input
- one canvas
- basic texture upload / draw flow or headless verification path
- no DLL execution
- optional muted audio

---

## Task 6: Define Compatibility And Package Implications

**Files:**
- Modify later: `crates/iwm-parser/src/models.rs`
- Modify later: `crates/iwm-parser/src/package_builder.rs`
- Modify later: package notes under `docs/notes/`

- [ ] **Step 1: Re-evaluate whether the current runtime package is still the right execution input**

Questions to answer:

- should WASM runtime consume current normalized JSON directly?
- should it consume a richer binary/package representation?
- should `scripts.ir.json` remain execution-facing, or become auxiliary diagnostics only?

- [ ] **Step 2: Preserve current outputs until the new runtime input is proven**

Do not prematurely delete:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `analysis.json`
- `resources/index.json`

These remain useful for inspection and debugging.

- [ ] **Step 3: Avoid binding the future runtime to TS-only assumptions**

Do not keep adding parser fields whose only consumer is the old TS gameplay runtime unless they also help diagnostics or the future WASM host.

---

## Task 7: Verification Gates

**Files:**
- Modify: none

- [ ] **Gate 1: Documentation consistency**

Verify that:

- README
- runtime notes
- gold sample note
- this WASM-first plan

all describe the same strategic runtime direction.

- [ ] **Gate 2: Headless core bring-up**

Verify that the extracted runtime core can:

- initialize
- load one sample
- run deterministic ticks

- [ ] **Gate 3: Browser WASM smoke**

Verify that a browser shell can:

- load the WASM module
- initialize the runtime host
- feed input or test ticks
- receive diagnostics or draw-state callbacks

- [ ] **Gate 4: Sample-driven priority list**

After the first smoke:

- rank remaining blockers by sample impact
- prioritize runner-semantic blockers over shell polish

---

## Workload And Risk Summary

### Compared to continuing the TS gameplay runtime

This WASM-first route:

- is slower to first visible playability
- is more aligned with the final fidelity target
- avoids duplicating a large amount of gameplay-runtime effort
- shifts complexity from heuristic gameplay emulation to host-boundary extraction

### Primary technical risks

- `gm8emulator` host assumptions are deep, not superficial
- render and window setup may be tightly interwoven with game startup
- audio and external calls may need broader abstraction than expected
- browser-safe support for externals/DLL behavior will remain intentionally limited

### Primary product/legal risk

- OpenGMK licensing may constrain how this runtime can be distributed

That risk must remain visible in every major runtime decision.

---

## Self-Review

Spec coverage for this plan:

- browser runtime direction: covered
- OpenGMK-centered execution strategy: covered
- host-boundary extraction: covered
- current TS runtime repositioning: covered
- final legal acceptance: not solved here, but explicitly surfaced

Expected outcome after this phase:

- the project stops drifting into a duplicate TS gameplay engine
- a concrete OpenGMK-to-WASM bring-up path exists
- the next runtime milestone is a headless/browser-hosted OpenGMK core, not a more elaborate heuristic runtime
