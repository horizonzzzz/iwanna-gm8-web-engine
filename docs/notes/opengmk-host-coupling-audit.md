# OpenGMK Host Coupling Audit

> **Maintenance note:** Update this file when the runtime host boundary changes enough that existing coupling guidance becomes misleading.

This note records the first host-boundary audit for the WASM-first runtime plan.

The goal of this pass is not to finish extraction. The goal is to identify which
parts of `vendor/OpenGMK/gm8emulator/` are tightly bound to desktop host services,
which parts are essential for a first browser boot, and where a narrow host trait
layer should sit.

Current repository route decision:

- OpenGMK `gm8emulator` is now the intended runtime-semantics mainline for Phase 4
- project-owned parser work remains responsible for extracting and structuring runtime input
- the repository should prefer host-boundary extraction plus headless bring-up over further TS gameplay-runtime growth

## Source Anchors

The main coupling points found in this pass are:

- `vendor/OpenGMK/gm8emulator/Cargo.toml`
- `vendor/OpenGMK/gm8emulator/src/main.rs`
- `vendor/OpenGMK/gm8emulator/src/game.rs`
- `vendor/OpenGMK/gm8emulator/src/render.rs`
- `vendor/OpenGMK/gm8emulator/src/input.rs`
- `vendor/OpenGMK/gm8emulator/src/game/audio.rs`
- `vendor/OpenGMK/gm8emulator/src/game/external.rs`
- `vendor/OpenGMK/gm8emulator/src/game/recording/`
- `vendor/OpenGMK/gm8emulator/src/imgui_utils.rs`

## Core Findings

### 1. `gm8emulator` is not currently shaped as a reusable library

`gm8emulator` is declared as a binary crate and its startup flow begins in
`src/main.rs`, which parses CLI arguments, reads the target EXE, constructs temp
directories, and then calls `Game::launch(...)`.

That means the current extraction target is not "compile an existing library to
WASM". The current extraction target is "separate reusable runtime state and host
services from a desktop-first binary entrypoint".

### 2. `Game::launch` mixes game boot with host setup

`Game::launch` in `src/game.rs` performs several unrelated responsibilities in a
single path:

- process and working-directory setup
- temp-directory selection
- included-file export
- compiler and asset registration
- `ramen` connection and window creation
- renderer construction
- external DLL setup
- audio manager creation

This is the main extraction choke point. A headless/browser-hosted runtime cannot
reuse `Game::launch` unchanged because the host-dependent setup is interleaved with
otherwise reusable game initialization.

### 3. Rendering is deeply tied to `ramen` windowing and OpenGL ownership

`src/render.rs` exposes a large rendering surface, but `Renderer::new(...)`
currently requires a `ramen::connection::Connection` and `ramen::window::Window`.

This is not just a draw-command sink. It is a desktop renderer with direct window
ownership assumptions, OpenGL backend setup, framebuffer management, and saved
renderer state.

For the first feasibility spike, a full browser renderer is unnecessary. A
headless/null render host is the right first target as long as boot and tick can
advance without real presentation.

### 4. Input translation is desktop-event driven

`src/input.rs` owns GM8-style key and mouse state, but it is fed through
`ramen::input::*` mappings and desktop event assumptions.

The useful reusable core here is the GameMaker-facing input state machine, not the
desktop event source. Browser and headless hosts should inject button states rather
than reuse `ramen` directly.

### 5. Audio currently assumes native output devices and optional ffmpeg capture

`src/game/audio.rs` creates a live audio session through `udon`, opens a default
output device, and optionally spawns `ffmpeg` for capture output.

That is too host-specific for the first browser boot. Audio should be abstracted
behind a small host interface and initially support a no-op implementation.

### 6. Externals/DLLs are already isolated enough to stub early

`src/game/external.rs` centralizes DLL/external behavior behind `ExternalManager`.
It already distinguishes native, IPC, emulated, and dummy behavior.

This is good news for the feasibility spike:

- the browser host can explicitly reject externals
- the headless host can preserve diagnostics about attempted definitions/calls
- dummy-audio DLL behavior can remain a targeted compatibility fallback

### 7. Recording/debug UI is a deferrable desktop concern

`src/game/recording/` plus `src/imgui_utils.rs` and clipboard dependencies are
important for the desktop emulator, but they are not required for a first browser
boot or deterministic tick loop.

These should be excluded from the first extraction target instead of being pulled
into the host boundary.

## Must Keep vs Can Defer

Must keep for the first browser/headless bring-up:

- GM8 asset loading and compilation path
- room loading and room state mutation
- object lifecycle and event dispatch
- collision and movement semantics
- GML/action execution
- deterministic clock/tick control
- explicit diagnostics for unsupported externals or host calls

Can defer safely in the first spike:

- desktop window creation and resize handling
- OpenGL presentation path
- clipboard integration
- record/replay windows and related imgui tooling
- ffmpeg capture
- native process spawning
- broad DLL compatibility
- real audio playback

## Immediate Blockers

| Area | Current Module | Why It Blocks Headless/WASM | First-Step Response |
| --- | --- | --- | --- |
| Entry point | `src/main.rs`, `src/game.rs` | Boot logic and host setup are interleaved | Split launch orchestration from host-specific setup |
| Windowing | `src/game.rs`, `src/render.rs` | `ramen` connection and `Window` are required during launch | Replace direct ownership with an injected render/window host |
| Rendering | `src/render.rs` | Renderer expects native GL/window lifecycle | Start with a null/headless render host |
| Input events | `src/input.rs` | Desktop event source is baked into mappings | Inject already-normalized button state from the host |
| Audio | `src/game/audio.rs` | Requires native output device and optional ffmpeg process | Start with a no-op audio host |
| Temp/included files | `src/game.rs` | File export and temp-dir policy are mixed into launch | Move file effects behind a constrained file host |
| DLLs/externals | `src/game/external.rs` | Browser cannot honor native DLL semantics | Reject explicitly and surface diagnostics |

## Minimal Host Surface For Phase 4A

The first host layer should stay narrow:

- `RuntimeTimeHost`
  - deterministic time source
  - fixed tick-rate metadata
- `RuntimeInputHost`
  - normalized button state queries
  - mouse position when needed
- `RuntimeRenderHost`
  - headless/null frame submission first
  - no browser texture API commitment yet
- `RuntimeAudioHost`
  - play/stop surface only
  - no-op implementation acceptable
- `RuntimeFileHost`
  - controlled reads
  - constrained temp writes/removals
- `RuntimeExternalHost`
  - explicit unsupported behavior
  - later bridge point for selective compatibility layers
- `RuntimeDiagnosticsHost`
  - collect structured runtime diagnostics

These boundaries are implemented initially in `crates/iwm-runtime-host/`.

## Recommended First Extraction Strategy

1. Keep `iwm-parser` and the current shell unchanged for now.
2. Treat `gm8emulator` as the semantic source of truth, but do not bind the browser
   path to its current binary entrypoint.
3. Introduce a headless host contract first, with null/no-op implementations.
4. Extract launch/game-state construction only after those host boundaries are fixed.
5. Delay browser rendering until deterministic boot/tick works in a headless Rust
   harness.

## Route Implication

Because the project is open source, the current engineering plan prefers semantic correctness over preserving a non-GPL runtime path at all costs.

That means:

- extracting or wrapping OpenGMK-derived semantics is now a chosen direction, not only a feasibility study
- licensing constraints remain real and must stay visible in release planning
- "reference, do not copy" still applies to direct code movement decisions, but the repository should no longer pretend that a shallow project-owned runtime is the likely end state

## Follow-Up Work

The next concrete implementation steps after this note are:

- keep the host traits intentionally small
- build a headless harness around those traits
- identify the smallest slice of `Game::launch` that can be separated without
  bringing `ramen` window ownership along
- only then start a browser/WASM adapter
