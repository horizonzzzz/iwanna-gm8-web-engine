# Minimal Playable Runtime Implementation Plan

> **Status note:** This document reflects the previous TS-first gameplay-runtime direction. It is now superseded in strategic direction by `docs/superpowers/plans/2026-05-20-opengmk-wasm-first-runtime.md`.
>
> This plan may still contain useful shell/UI/diagnostics tasks, but gameplay-engine work should no longer use this document as the primary implementation route.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Advance the current runtime shell from static room inspection to a first playable browser runtime that can boot one gold-sample IWanna game and support core interaction: entering a room, moving, jumping, colliding, dying, respawning, and performing a basic room transition.

**Architecture:** Keep the current division of responsibilities. Rust continues to parse GM8 packages and emit a runtime-facing package. The browser runtime remains project-owned and executes a deliberately narrow compatibility subset from normalized runtime data plus `scripts.ir.json`. Do not pivot to a WASM runner in this phase.

**Important scope note:** This phase is about one playable gold sample, not broad GM8 compatibility. Do not attempt to support arbitrary GML, broad DLL semantics, particles, surfaces, menus, save systems, or every event type. The runtime should execute only the minimum subset required to make one gold sample playable.

**Current constraint note:** The package format exists and the shell can already render static rooms. However, runtime execution is still explicitly incomplete: `analysis.json` currently emits warnings such as `logic-execution-not-yet-implemented` and `room-runtime-not-yet-implemented`, and parser output still contains both `action-list` and `source-only` script blocks. This plan must reduce those gaps for one playable target before trying to generalize further.

**Tech Stack:** Rust 1.77+, Cargo workspace, `serde`, `serde_json`, vendored `gm8exe` via tracked submodule, Node 20+, Vite, TypeScript, Canvas 2D, Vitest

---

## File Structure

Planned files for this phase:

- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `crates/iwm-parser/src/models.rs`
- Modify: `crates/iwm-parser/src/logic_export.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Create: `docs/notes/runtime-gold-sample.md`
- Create: `runtime/src/runtime/types.ts`
- Create: `runtime/src/runtime/fixedStepLoop.ts`
- Create: `runtime/src/runtime/input.ts`
- Create: `runtime/src/runtime/instanceState.ts`
- Create: `runtime/src/runtime/collision.ts`
- Create: `runtime/src/runtime/eventDispatch.ts`
- Create: `runtime/src/runtime/logicRunner.ts`
- Create: `runtime/src/runtime/roomState.ts`
- Create: `runtime/src/runtime/gameRuntime.ts`
- Create: `runtime/src/runtime/fixedStepLoop.test.ts`
- Create: `runtime/src/runtime/collision.test.ts`
- Create: `runtime/src/runtime/logicRunner.test.ts`
- Create: `runtime/src/runtime/gameRuntime.test.ts`
- Modify: `runtime/src/types.ts`
- Modify: `runtime/src/loadPackage.ts`
- Modify: `runtime/src/render/resourceCache.ts`
- Modify: `runtime/src/render/staticRoomRenderer.ts`
- Modify: `runtime/src/ui/inspectors.ts`
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.ts`
- Modify: `runtime/src/main.test.ts`
- Modify: `runtime/src/styles.css`

Responsibilities:

- `iwm-parser/src/models.rs`: extend the runtime package contract only where the browser runtime truly needs more explicit semantic data
- `iwm-parser/src/logic_export.rs`: improve IR lowering for the minimal playable subset and expose block metadata needed for runtime dispatch
- `iwm-parser/src/package_builder.rs`: surface better runtime capability warnings and support-level summaries
- `docs/notes/runtime-gold-sample.md`: capture the exact gold sample, why it was chosen, and which gameplay behaviors are in-scope for this phase
- `runtime/src/runtime/`: hold the browser-side execution core; keep it separate from shell UI and rendering helpers
- `runtime/src/ui/shell.ts`: evolve from “static viewer” into a simple dev harness that can boot, pause, reset, and inspect runtime state

## Preconditions

Before starting this phase:

- Phase 3 code should already pass `cargo test`
- `npm --prefix runtime test` and `npm --prefix runtime run build` should already pass
- the tracked `vendor/OpenGMK/` submodule must be initialized for parser work
- the package format under `docs/notes/package-format-v1-runtime.md` remains the source of truth for current runtime package outputs
- at least one gold-sample package should already be generatable under `runtime/public/packages/`

## Gold Sample Strategy

This phase should explicitly choose a gold sample and a fallback sample.

Recommended primary gold sample:

- `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

Recommended fallback or comparison sample:

- `samples/local/iwanna-examples/gm8-core/I Wanna Kill the Kamilia Ver. Final`

Reasoning:

- `IWBT_Dife` is already used in existing smoke tests and package generation
- `IWBT_Dife` still contains a meaningful number of `source-only` blocks, so it is a good forcing function for the parser/runtime boundary
- `Kamilia` currently appears to produce mostly `action-list` blocks, which makes it useful as a contrast sample when debugging IR execution

The plan target is not “fully beatable.” The target is:

- boot one sample
- enter gameplay
- move and jump
- collide with solid terrain
- die on basic hazards
- respawn correctly
- traverse at least one room transition

---

## Task 1: Document Phase 4 Runtime Scope

**Files:**
- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Create: `docs/notes/runtime-gold-sample.md`

- [ ] **Step 1: Add a short phase summary to the README**

Update the runtime section so it is clear that:

- Phase 3 is complete
- the next active milestone is “minimal playable runtime”
- the current shell is no longer only a static viewer target

- [ ] **Step 2: Extend the runtime package note with current execution limitations**

Add a section to `docs/notes/package-format-v1-runtime.md` that distinguishes:

- currently runtime-consumable static data
- currently executable action-list subset
- still deferred `source-only` / unsupported runtime features

- [ ] **Step 3: Add `docs/notes/runtime-gold-sample.md`**

Document:

- chosen gold sample path
- why it was chosen
- expected milestone behaviors
- out-of-scope features for this phase
- known risky mechanics or rooms

- [ ] **Step 4: Run a doc sanity pass**

Check that:

- the README, package note, and gold-sample note do not contradict each other
- they do not claim broad playability or generic GM8 support yet

---

## Task 2: Tighten The Runtime Execution Contract

**Files:**
- Modify: `crates/iwm-parser/src/models.rs`
- Modify: `crates/iwm-parser/src/logic_export.rs`
- Modify: `crates/iwm-parser/src/package_builder.rs`
- Modify: `crates/iwm-parser/tests/build_package_smoke.rs`
- Modify: `runtime/src/types.ts`
- Modify: `runtime/src/loadPackage.ts`

- [ ] **Step 1: Identify the runtime metadata still missing from current package outputs**

Review whether the browser runtime needs explicit data for:

- event names or normalized event tags
- object inheritance lookup
- room transition targets
- solid / hazard categorization heuristics
- spawn / save point metadata if discoverable

Only add fields that materially reduce browser-side guesswork.

- [ ] **Step 2: Add failing tests for any new contract fields**

Examples:

- event table entries preserving enough data to dispatch create / step / collision blocks
- script block metadata making runtime selection easier than raw numeric event ids
- room or object data exposing fields currently buried in implicit GM8 assumptions

- [ ] **Step 3: Keep `scripts.ir.json` narrow but more executable**

Improve `logic_export.rs` so that the browser runtime gets:

- stable block ids
- a clearer distinction between executable `action-list` blocks and fallback `source-only` blocks
- enough metadata to decide whether a room can enter the playable path or should degrade with diagnostics

- [ ] **Step 4: Improve analysis warnings**

Replace or supplement the current generic warnings with more actionable output, for example:

- `runtime-missing-source-lowering`
- `runtime-unsupported-event:<type>`
- `runtime-unsupported-action:<fn_name>`

- [ ] **Step 5: Mirror the contract changes in `runtime/src/types.ts` and loader tests**

Keep Rust and TypeScript models aligned.

- [ ] **Step 6: Run targeted parser tests**

Run:

```bash
cargo test -p iwm-parser
```

Expected:

```text
test result: ok
```

---

## Task 3: Build The Runtime Core Skeleton

**Files:**
- Create: `runtime/src/runtime/types.ts`
- Create: `runtime/src/runtime/fixedStepLoop.ts`
- Create: `runtime/src/runtime/input.ts`
- Create: `runtime/src/runtime/instanceState.ts`
- Create: `runtime/src/runtime/roomState.ts`
- Create: `runtime/src/runtime/eventDispatch.ts`
- Create: `runtime/src/runtime/gameRuntime.ts`
- Create: `runtime/src/runtime/fixedStepLoop.test.ts`
- Create: `runtime/src/runtime/gameRuntime.test.ts`

- [ ] **Step 1: Add a failing runtime bootstrap test**

Write a test that proves a runtime instance can:

- accept a loaded package
- boot into the manifest default room
- create room instances
- expose an update tick without throwing

- [ ] **Step 2: Create runtime-facing state types**

Define runtime-only structures for:

- current room state
- live instances
- global runtime state
- input snapshot
- runtime diagnostics

Keep these separate from package schema types.

- [ ] **Step 3: Add a fixed-step loop helper**

Requirements:

- deterministic step size
- manual tick mode for tests
- pause / resume support
- frame accumulator bounded to avoid runaway catch-up

- [ ] **Step 4: Add input state handling**

Start with keyboard support for:

- left
- right
- jump
- restart if needed for debugging

The runtime should consume normalized input state, not raw DOM events directly.

- [ ] **Step 5: Add room bootstrapping**

When a room loads:

- create live instances from `rooms.json`
- resolve object definitions
- record room dimensions and active views
- dispatch create events in a deterministic order

- [ ] **Step 6: Add runtime lifecycle tests**

Test:

- boot into default room
- reload current room
- perform a room reset
- keep instance counts stable across expected transitions

---

## Task 4: Execute The Action-List Subset

**Files:**
- Create: `runtime/src/runtime/logicRunner.ts`
- Create: `runtime/src/runtime/logicRunner.test.ts`
- Modify: `runtime/src/runtime/eventDispatch.ts`
- Modify: `runtime/src/runtime/gameRuntime.ts`

- [ ] **Step 1: Choose the first supported `LogicOp` subset**

Prioritize support for the smallest set that unlocks playable IWanna behavior:

- variable reads and writes for instance-local state
- arithmetic updates
- simple comparisons and conditions
- common movement-related action calls
- room restart / room goto when expressed as action-list calls
- object creation / destruction if the gold sample needs them immediately

- [ ] **Step 2: Add failing tests for `ActionCall` execution**

Test at least:

- a state mutation action
- a conditional branch action
- a room reset or death-trigger action
- one unsupported action producing a runtime diagnostic instead of a silent failure

- [ ] **Step 3: Implement a minimal action dispatcher**

Do not attempt a full generic GM8 function layer. Start with a registry of explicitly supported actions needed by the gold sample.

- [ ] **Step 4: Define unsupported behavior policy**

For unsupported blocks:

- collect runtime diagnostics
- fail closed only when the unsupported block is on the critical path
- keep the dev shell explicit about what was skipped

- [ ] **Step 5: Treat `source-only` blocks as first-class blockers**

For this phase, do one of:

- lower a tiny subset of frequent source snippets into explicit IR, or
- classify specific `source-only` blocks as unsupported-but-noncritical, while keeping the gold path playable

The decision must be driven by the actual gold sample, not by aesthetics.

---

## Task 5: Add Collision, Movement, And Death

**Files:**
- Create: `runtime/src/runtime/collision.ts`
- Create: `runtime/src/runtime/collision.test.ts`
- Modify: `runtime/src/runtime/gameRuntime.ts`
- Modify: `runtime/src/runtime/logicRunner.ts`

- [ ] **Step 1: Add a failing collision test**

At minimum:

- horizontal solid collision stops movement
- vertical collision supports floor landing
- simple hazard contact can trigger death

- [ ] **Step 2: Model the first collision rules**

Requirements:

- axis-separated or otherwise deterministic collision resolution
- explicit solid and hazard lookup strategy
- reproducible behavior in tests

Avoid overengineering generalized physics. This is a platformer compatibility layer, not a physics sandbox.

- [ ] **Step 3: Add the first movement model**

For the player-compatible object path, support:

- horizontal input
- gravity
- jump initiation
- floor detection
- simple respawn or room reset on death

The exact values can initially come from sample-driven heuristics if the parser does not yet expose all movement constants directly.

- [ ] **Step 4: Add death and respawn flow**

Support at least one of these paths, whichever matches the gold sample:

- room restart
- spawn-point reset within current room

- [ ] **Step 5: Add runtime diagnostics around collision and death**

Record:

- missing collision masks
- missing player candidate
- death trigger with no respawn path

---

## Task 6: Add Room Transition And A Playable Shell

**Files:**
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/ui/inspectors.ts`
- Modify: `runtime/src/main.ts`
- Modify: `runtime/src/styles.css`
- Modify: `runtime/src/render/staticRoomRenderer.ts`

- [ ] **Step 1: Add a shell test for boot / pause / reset controls**

The developer shell should let you:

- load package
- boot runtime
- pause or resume stepping
- reset current room

- [ ] **Step 2: Move the renderer from “static room draw” toward “runtime frame draw”**

The same canvas should now render:

- current live room state
- live instance positions
- current frame of visible instances

Still keep the rendering path simple and deterministic.

- [ ] **Step 3: Add room transition support**

Minimum requirement:

- support transition into another room when triggered through the supported action subset
- rebuild room state correctly
- preserve only the runtime state that should persist

- [ ] **Step 4: Surface runtime diagnostics in the shell**

Add a visible diagnostics panel for:

- unsupported blocks encountered
- room transitions
- deaths and respawns
- runtime action failures

- [ ] **Step 5: Keep manual room selection only as a dev override**

The shell can still expose room selection, but it should no longer be the primary way to move between rooms once runtime transitions exist.

---

## Task 7: End-To-End Verification For One Playable Sample

**Files:**
- Modify: none

- [ ] **Step 1: Run Rust tests**

```bash
cargo test
```

- [ ] **Step 2: Run frontend tests**

```bash
npm --prefix runtime test
```

- [ ] **Step 3: Build the runtime app**

```bash
npm --prefix runtime run build
```

- [ ] **Step 4: Generate the gold-sample runtime package**

```bash
cargo run -p iwm-cli -- build-package --input ".\\samples\\local\\iwanna-examples\\gm8-core\\IWBT_Dife" --output ".\\runtime\\public\\packages\\iwbt-dife"
```

- [ ] **Step 5: Launch the shell and verify the gameplay path manually**

```bash
npm --prefix runtime run dev -- --host 127.0.0.1
```

Manual verification target:

- package loads successfully
- runtime boots into the expected room
- player can move left and right
- player can jump
- player can die from at least one basic hazard
- player respawns correctly
- at least one room transition works
- diagnostics remain explicit when unsupported logic is encountered

- [ ] **Step 6: Document observed gaps**

Record:

- unsupported action names still encountered
- `source-only` blocks that remain on the critical path
- sample-specific quirks that should shape the next compatibility phase

---

## Self-Review

Spec coverage for this plan:

- fixed-step runtime loop: covered
- keyboard input: covered
- room loading and transitions: covered
- collision and movement: covered
- death and respawn: covered
- script execution for a narrow subset: covered
- broad GM8 compatibility: intentionally deferred
- WASM runner pivot: intentionally deferred

Sample-driven rules:

- do not optimize for multiple games before one gold sample is playable
- do not broaden supported logic ops until diagnostics show a clear next need
- do not let runtime heuristics leak into parser output without an explicit contract reason

Expected outcome after this phase:

- one gold-sample game is minimally playable in-browser using the project-owned runtime
- the parser/runtime boundary is clearer
- the next compatibility phase can focus on expanding executable IR coverage rather than inventing the runtime architecture from scratch
