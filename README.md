# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phase

Phase 4 has switched to a WASM-first runtime strategy.

The current `runtime/` app remains the browser shell, package inspector, and diagnostics harness, but the long-term gameplay execution path is no longer the project-owned TypeScript runtime. Runtime fidelity work now targets adapting OpenGMK `gm8emulator` into a browser-hosted WASM execution core.

Phase 4 route decision is now explicit:

- runtime mainline: adapt or extract an OpenGMK-derived execution core behind project-owned host boundaries, then run it through the browser-facing WASM path
- parser mainline: replace the current shallow GML lowering path with a real parser-owned expression/statement model that can preserve callable structure, variable references, and array/member access for the runtime contract
- deprecated direction: do not keep expanding the old TS-side gameplay runtime as if it were the long-term engine

This project is open source and now assumes a GPL-2.0-compatible direction for the runtime path unless a later architecture change explicitly removes that dependency. OpenGMK usage is already part of the repository's architecture decision surface, so license validation is a release-blocking requirement rather than a follow-up legal nicety. That does **not** authorize casual code copying from `vendor/OpenGMK/`; it means the repository now treats OpenGMK coupling and licensing as an explicit architectural dependency to manage, not a reason to continue investing in a semantically weak fallback runtime.

Current implemented Phase 4 slices:

- `crates/iwm-runtime-model/` holds the shared runtime package schema
- `crates/iwm-runtime-host/` defines the first host-boundary traits and headless helpers
- `crates/iwm-runtime-core/` provides a deterministic headless runtime-core skeleton
- `crates/iwm-runtime-web/` exposes a browser-loadable WASM bridge surface
- `runtime/` can load a normalized package, probe `/wasm/iwm_runtime_web.wasm`, drive the bridge for boot/tick/diagnostics, submit keyboard input, and draw returned frame commands onto the browser canvas
- runtime package resources now carry gm8exe-derived sprite collision masks, and runtime-core uses them for pixel-level collision checks after bbox broad-phase filtering

Phase 3 is complete and delivered the runtime-facing package format and development shell with static room viewer.

## Overview

This project explores a practical path for running mainstream legacy IWanna fangames in the browser.

The intended pipeline is:

1. accept an original game package
2. detect whether it is likely a supported GM8-style target
3. parse the package on the backend
4. normalize it into a project-owned package format
5. run that package in a browser runtime

The goal is not to emulate every Game Maker game. The first focus is a narrower compatibility subset that can support core IWanna gameplay.

## Current Phases

- Phase 1: detector foundation
- Phase 2: GM8 parser adapter and normalized package builder
- Phase 3: runtime-facing package format and development static room viewer (complete)
- Phase 4: OpenGMK WASM-first runtime bring-up (in progress)

See `docs/notes/package-format-v1-runtime.md` for the current runtime package contract.
See `docs/notes/runtime-wasm-gap-analysis.md` for the current checklist of what is still missing for a fully playable WASM runtime.
See `docs/notes/runtime-vendor-reference-map.md` for the current OpenGMK-guided runtime reference map.
See `docs/notes/opengmk-host-coupling-audit.md` for the first OpenGMK host-boundary audit.
See `docs/notes/runtime-gold-sample.md` for the active gold-sample validation target.

## Documentation Notes

Current-state documents should be read as the primary project guide:

- `README.md`
- `AGENTS.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`

Implementation plans are intentionally not kept in-repo. Use the current specs, notes, and actual repository state instead.

## Setup

```powershell
git submodule update --init --recursive
npm --prefix runtime install
rustup target add wasm32-unknown-unknown
```

On Windows, build the WASM target from a Visual Studio Developer Command Prompt or otherwise ensure `clang` and `clang++` are on `PATH`.

## Use And Test Now

### 1. Verify Rust and frontend tests

```powershell
cargo test
npm --prefix runtime test
npm --prefix runtime run test:browser
npm --prefix runtime run build
```

The browser smoke covers the shell-visible runtime telemetry path:

- WASM boot status
- current room label
- current tick
- player availability summary
- diagnostic summary

The runtime package now includes both:

- `logic.raw.json` for parser-owned raw GML preservation
- `logic.lowered.json` for the current parser-owned lowered logic contract

The browser shell now loads those files alongside `manifest.json`, `rooms.json`, `objects.json`, `scripts.ir.json`, `analysis.json`, and `resources/index.json`.

### 2. Build and sync the WASM bridge

```powershell
$env:PATH='C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\Llvm\bin;' + $env:PATH
$env:CC='clang'
$env:CXX='clang++'
cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
npm --prefix runtime run sync:wasm
```

This produces `target\wasm32-unknown-unknown\debug\iwm_runtime_web.wasm` and copies it to `runtime\public\wasm\iwm_runtime_web.wasm` for the browser shell.

### 3. Build a runtime package if you have a local sample

```powershell
cargo run -p iwm-cli -- detect --input C:\path\to\game
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample
```

The shell default package path is `/packages/sample`, which maps to `runtime\public\packages\sample\`.

### 4. Launch the browser shell

```powershell
npm --prefix runtime run dev -- --host 127.0.0.1
```

Then open `http://127.0.0.1:4173`.

Current browser controls for the WASM runtime path:

- click `Load Package`
- use `ArrowLeft` / `A` for left
- use `ArrowRight` / `D` for right
- use `Space` / `ArrowUp` / `W` for jump
- use `R` for restart
- the WASM path now auto-runs at 60 Hz; use `Pause` to pause and `Resume` to continue

Important local-only paths:

- `runtime/public/packages/` is intentionally empty in git except for `.gitkeep`
- `runtime/public/wasm/iwm_runtime_web.wasm` is a generated local artifact and is not committed
- `samples/local/iwanna-examples/` is a local sample area and may not exist in a fresh clone

## Repository Contents

- `docs/`
  Project documentation and design notes
- `runtime/`
  Browser shell, package loader, diagnostics UI, and WASM bridge glue
- `samples/local/iwanna-examples/`
  Local sample corpus used for detector and parser validation
- `vendor/`
  Tracked upstream reference submodules used for GM8 format study and parser research

Current workspace crates include:

- `crates/iwm-detector/`
- `crates/iwm-parser/`
- `crates/iwm-cli/`
- `crates/iwm-runtime-model/`
- `crates/iwm-runtime-host/`
- `crates/iwm-runtime-core/`
- `crates/iwm-runtime-web/`

Current runtime crate layout notes:

- `crates/iwm-runtime-host/` separates host-boundary types and traits from default implementations such as clock, input, file, render, external, diagnostics, and headless host composition
- `crates/iwm-runtime-web/` separates bridge-facing models, runtime-host wrapper logic, translation helpers, result storage, and exported WASM FFI entrypoints
- `crates/iwm-runtime-core/` separates runtime types, top-level orchestration, room building, room transitions, movement, lowered logic execution, rendering, diagnostics, and crate-local test support

Later planned areas include:

- `backend/`

## Sample Corpus

The project-local sample corpus is organized under `samples/local/iwanna-examples/` when populated locally.

Current categories:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Suggested usage:

- start smoke testing with `gm8-core`
- use `non-target` for negative classification checks
- treat current labels as working development categories, not final truth
- prefer repo-local sample paths in scripts and plans instead of old desktop paths
- expect a fresh clone to omit actual sample binaries unless you add them locally

## Vendored References

The `vendor/` directory is used for upstream study and narrow integration experiments.

Important repository rule:

- `vendor/README.md` is tracked
- upstream repositories under `vendor/` are tracked as git submodules
- clone with submodules or run `git submodule update --init --recursive` after checkout

Current references:

- `OpenGMK`
- `GM8Decompiler`

These references are useful for:

- studying `gm8exe`
- validating GM8 parsing assumptions
- checking edge cases in legacy executable handling

> [!CAUTION]
> Some OpenGMK ecosystem components may be `GPL-2.0-only`. Any direct dependency or code reuse should be treated as a deliberate licensing decision.

## Scope

The current project direction is centered on:

- detecting likely GM8-style IWanna fangame packages
- parsing targetable GM8 executables and related resources
- building a normalized project-owned package format
- preparing for a browser runtime that can execute core gameplay through a WASM-hosted engine path

For historical context, the Phase 2 package-builder milestone emitted a structural V0 package consisting of:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.json`
- `analysis.json`

That V0 output has been superseded by the current runtime-facing Phase 3 package, which now includes browser-ready `resources/` exports and `scripts.ir.json`.

Important runtime direction note:

- the current package and frontend shell remain useful
- the removed TypeScript gameplay runtime should be treated as transitional tooling that no longer participates in the active browser execution path
- future runtime-fidelity work should accumulate in the OpenGMK-derived WASM-hosted engine path, not in a parallel TS gameplay reimplementation
- parser work should now focus on turning raw GML and shallow lowered snippets into a real runtime-facing contract rather than emitting strings that only the old TS runtime could heuristically inspect
- when the WASM bridge is missing, unsynced, or fails to boot, the current shell falls back to a static room viewer instead of a gameplay runtime

## Phase 4 Priorities

The next development direction is intentionally split into two coupled tracks.

### Runtime track

- extract or wrap OpenGMK `gm8emulator` semantics behind narrow host traits
- prove headless/null-host boot before deeper browser rendering work
- keep browser work focused on WASM host integration, diagnostics, and controls

### Parser track

- keep the parser-owned lowered contract as the runtime-facing source of executable structure for the current IWanna-critical subset
- preserve real call, member, index, and binary-expression structure so runtime code executes semantics instead of guessing from raw strings
- treat `logic.raw.json` as preservation and diagnostics data, not as the steady-state execution contract
- extend the lowered contract only when gold-sample evidence shows that the current structured subset is insufficient on the critical path

### Near-Term Execution Order

The next development cycle should execute in this order:

1. keep the shared lowered parser contract stable except where gold-sample evidence requires targeted expansion
2. headless OpenGMK-derived runtime extraction behind narrow host traits
3. browser host follow-through for that runtime core
4. audio, animation, and broader lifecycle coverage after the runtime can execute trustworthy semantics

### Practical decision rule

- if a task improves shell UX or telemetry without affecting engine semantics, it can stay in `runtime/`
- if a task tries to reimplement more GM8 gameplay behavior in TS, it is usually the wrong direction now
- if a task clarifies parser-owned runtime data or reduces OpenGMK host coupling, it is aligned with the current plan

Out of scope for the MVP:

- broad support for all Game Maker games
- non-GM engines
- full engine parity from day one

## Notes

- Local sample files should be treated as development assets, not canonical source files
- Do not redistribute copyrighted game binaries casually
- Multi-file packages are not automatically non-targets; many GM8 games ship with DLL, audio, and config files
