# Vendor-Guided WASM Runtime Next Steps Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Use the vendored OpenGMK and GM8Decompiler references to drive the next runtime milestones without reopening the removed TypeScript gameplay runtime path.

**Architecture:** Treat `vendor/OpenGMK/gm8emulator/` as the semantic reference for runtime behavior and `vendor/OpenGMK/gm8exe/` as the parser-facing data source. Treat `vendor/GM8Decompiler/` only as a parser-validation and odd-sample comparison reference. Keep the current normalized package and browser shell in place until a concrete blocker proves that the package must evolve.

**Tech Stack:** Rust workspace crates, vendored `OpenGMK` / `GM8Decompiler` submodules, `gm8exe`, current `iwm-runtime-core` / `iwm-runtime-host` / `iwm-runtime-web`, Vite runtime shell

---

## Reference Direction

Use the vendor repositories with these boundaries:

- `vendor/OpenGMK/gm8emulator/src/game/movement.rs`
  Runtime movement, gravity, jump, and collision semantics reference
- `vendor/OpenGMK/gm8emulator/src/game/events.rs`
  Object lifecycle and event ordering reference
- `vendor/OpenGMK/gm8emulator/src/game/transition.rs`
  Room transition and restart behavior reference
- `vendor/OpenGMK/gm8emulator/src/input.rs`
  GameMaker-facing input model reference
- `vendor/OpenGMK/gm8emulator/src/render.rs` and `src/game/draw.rs`
  Draw ordering and frame-shape reference
- `vendor/OpenGMK/gm8exe`
  Parser and asset-structure reference
- `vendor/GM8Decompiler`
  Parser-result comparison only; do not use it to define runtime behavior

Non-goals for this plan:

- restoring the deleted TS gameplay runtime
- redesigning the runtime package before a hard blocker appears
- broad DLL/audio/menu parity work

---

### Task 1: Build A Vendor Reference Matrix

**Files:**
- Create: `docs/notes/runtime-vendor-reference-map.md`
- Read: `vendor/README.md`
- Read: `vendor/OpenGMK/gm8emulator/src/game/movement.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/game/events.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/game/transition.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/input.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/render.rs`
- Read: `vendor/GM8Decompiler/README.org`

- [ ] **Step 1: Record which vendor module owns which runtime concern**

Capture a table with:

- concern
- OpenGMK source file
- why it matters to this repo
- whether it is needed now, later, or only for validation

- [ ] **Step 2: Record the explicit “do not use this for semantics” rule for GM8Decompiler**

Write into the note:

- GM8Decompiler is for parser recovery behavior and odd executable comparisons
- OpenGMK `gm8emulator` is the runtime semantics source of truth
- `gm8exe` remains the only intended direct dependency boundary for parser code

- [ ] **Step 3: Commit the reference map**

Run:

```powershell
rtk git add docs/notes/runtime-vendor-reference-map.md
rtk git commit -m "docs: map vendor runtime references"
```

---

### Task 2: Audit Gold-Sample Blockers Against The Current Package

**Files:**
- Modify: `docs/notes/runtime-gold-sample.md`
- Read: `runtime/public/packages/mashikaku/analysis.json`
- Read: `runtime/public/packages/mashikaku/scripts.ir.json`
- Read: `runtime/public/packages/kamilia/analysis.json`
- Read later when available: `runtime/public/packages/sample/analysis.json`
- Read later when available: `runtime/public/packages/sample/scripts.ir.json`

- [ ] **Step 1: Categorize current blockers by layer**

Add a section that separates blockers into:

- parser missing data
- runtime-core semantic gap
- wasm/web host gap
- shell-only issue

- [ ] **Step 2: For each gold/comparison sample, identify the first room and first critical blocker**

Record:

- package path
- boot room id/name
- whether frame draws
- whether player appears
- whether movement works
- first blocking warning or missing behavior

- [ ] **Step 3: Explicitly mark which `source-only` warnings are on the critical path**

Do not treat every `runtime-missing-source-lowering:*` warning as equally urgent.
Mark only the blocks that prevent:

- first room boot
- visible player spawn
- left/right/jump
- death/reset
- room transition

- [ ] **Step 4: Commit the blocker note**

Run:

```powershell
rtk git add docs/notes/runtime-gold-sample.md
rtk git commit -m "docs: classify runtime gold-sample blockers"
```

---

### Task 3: Implement Runtime-Core Semantic Slice 1 From OpenGMK References

**Files:**
- Modify: `crates/iwm-runtime-core/src/lib.rs`
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Test: `crates/iwm-runtime-core/src/lib.rs` inline tests
- Read: `vendor/OpenGMK/gm8emulator/src/game/movement.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/game/events.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/game/transition.rs`
- Read: `vendor/OpenGMK/gm8emulator/src/input.rs`

- [ ] **Step 1: Write failing tests for minimal playable semantics**

Add targeted tests for:

- player spawn in default room
- left/right input changes player position
- jump input changes vertical state
- solid collision stops penetration
- hazard contact triggers reset/death diagnostic
- room restart returns to room spawn/checkpoint
- room transition request reloads target room

- [ ] **Step 2: Run the narrow failing test set**

Run:

```powershell
rtk cargo test -p iwm-runtime-core core_
```

Expected:

- movement/collision/reset tests fail because the current core only increments ticks and renders frames

- [ ] **Step 3: Implement the minimal state and stepping model**

Mirror the shape of OpenGMK responsibilities, but do not copy large upstream code.
The first implementation should add:

- persistent player state in `RuntimeCore`
- normalized button-to-intent mapping
- per-tick position updates
- simple solid and hazard checks
- reset / reload room path
- diagnostics for unsupported or deferred cases

- [ ] **Step 4: Re-run runtime-core tests**

Run:

```powershell
rtk cargo test -p iwm-runtime-core
```

Expected:

- all runtime-core tests pass

- [ ] **Step 5: Commit the runtime-core slice**

Run:

```powershell
rtk git add crates/iwm-runtime-core/src/lib.rs crates/iwm-runtime-host/src/lib.rs
rtk git commit -m "feat(runtime-core): add first vendor-guided gameplay semantics"
```

---

### Task 4: Thread The New Semantics Through The WASM Bridge And Shell

**Files:**
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Modify: `runtime/src/runtime/wasmSession.ts`
- Modify: `runtime/src/ui/shell.ts`
- Test: `crates/iwm-runtime-web/src/lib.rs` inline tests
- Test: `runtime/src/runtime/wasmBridge.test.ts`
- Test: `runtime/src/runtime/wasmSession.test.ts`
- Test: `runtime/src/main.test.ts`

- [ ] **Step 1: Write failing bridge tests for movement/reset/transition snapshots**

Cover:

- player position changes after tick with input
- reset returns to initial room state
- room switch produces a fresh frame
- diagnostics remain human-readable in the shell

- [ ] **Step 2: Run the failing bridge and frontend tests**

Run:

```powershell
rtk cargo test -p iwm-runtime-web
rtk npm --prefix runtime test
```

Expected:

- tests fail until the bridge exposes the updated runtime state cleanly

- [ ] **Step 3: Update the bridge and shell with the new core behavior**

Keep the shell contract unchanged where possible:

- WASM remains the only gameplay execution path
- static room viewer remains the fallback
- shell diagnostics explain whether a failure is semantic, package, or host-related

- [ ] **Step 4: Rebuild, sync, and verify the browser path**

Run in order:

```powershell
rtk cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
rtk npm --prefix runtime run sync:wasm
rtk npm --prefix runtime test
rtk npm --prefix runtime run build
```

Expected:

- tests pass
- browser shell still boots and renders
- player movement is now driven by the WASM runtime path

- [ ] **Step 5: Commit the bridge/shell slice**

Run:

```powershell
rtk git add crates/iwm-runtime-web/src/lib.rs runtime/src/runtime/wasmBridge.ts runtime/src/runtime/wasmSession.ts runtime/src/ui/shell.ts runtime/src/runtime/wasmBridge.test.ts runtime/src/runtime/wasmSession.test.ts runtime/src/main.test.ts
rtk git commit -m "feat(runtime): surface vendor-guided core semantics in wasm shell"
```

---

### Task 5: Re-Evaluate The Package Only If The Core Hits A Hard Blocker

**Files:**
- Modify only if needed: `crates/iwm-parser/src/models.rs`
- Modify only if needed: `crates/iwm-parser/src/package_builder.rs`
- Modify only if needed: `crates/iwm-parser/src/logic_export.rs`
- Modify only if needed: `docs/notes/package-format-v1-runtime.md`
- Read: `vendor/OpenGMK/gm8exe`
- Compare with: `vendor/GM8Decompiler`

- [ ] **Step 1: Define the blocker threshold before changing the package**

Only change the package if one of these is true:

- runtime-core needs data that `gm8exe` already exposes but the package drops
- a critical gold-sample mechanic cannot be expressed from the current package
- `scripts.ir.json` omits required event or action structure for the current milestone

- [ ] **Step 2: Prefer additive fields over format replacement**

If a package change is needed:

- add only the missing field(s)
- keep `manifest.json`, `rooms.json`, `objects.json`, `analysis.json`, and `resources/index.json`
- avoid deleting or renaming existing browser-shell inputs in the same change

- [ ] **Step 3: Use GM8Decompiler only as a comparison tool**

If a sample looks wrong:

- compare parser output against GM8Decompiler-recovered structure
- fix the parser/export boundary
- do not treat GM8Decompiler project-file output as the new runtime input format

- [ ] **Step 4: Commit package changes separately if they are required**

Run:

```powershell
rtk git add crates/iwm-parser/src/models.rs crates/iwm-parser/src/package_builder.rs crates/iwm-parser/src/logic_export.rs docs/notes/package-format-v1-runtime.md
rtk git commit -m "feat(parser): extend runtime package for wasm core blocker"
```

---

### Task 6: Keep A Standing Verification Loop

**Files:**
- Modify as needed: `docs/notes/runtime-gold-sample.md`
- Read: `runtime/public/packages/*`

- [ ] **Step 1: Verify the narrowest relevant layer first**

Use this order:

1. `rtk cargo test -p iwm-runtime-core`
2. `rtk cargo test -p iwm-runtime-web`
3. `rtk npm --prefix runtime test`
4. `rtk cargo build -p iwm-runtime-web --target wasm32-unknown-unknown`
5. `rtk npm --prefix runtime run sync:wasm`
6. live browser smoke on `http://localhost:4173/`

- [ ] **Step 2: Keep browser checks sample-driven**

Primary target:

- `IWBT_Dife`

Secondary checks:

- `Kamilia`
- `mashikaku`

- [ ] **Step 3: After each slice, record the next highest blocker**

Prefer documenting:

- missing runtime semantic
- host-bound blocker
- package field gap

Over vague labels like "still broken" or "needs more compatibility work".

---

## Expected Outcome

If this plan is followed, the next phase stays disciplined:

- OpenGMK drives runtime semantics
- GM8Decompiler stays in the parser-validation lane
- the browser shell remains stable and simple
- package format changes happen only when a proven runtime-core blocker demands them
- the repo moves toward one real WASM gameplay path instead of growing parallel execution systems
