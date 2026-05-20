# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phase

Phase 4 has switched to a WASM-first runtime strategy.

The current `runtime/` app remains the browser shell, package inspector, and diagnostics harness, but the long-term gameplay execution path is no longer the project-owned TypeScript runtime. Runtime fidelity work now targets adapting OpenGMK `gm8emulator` into a browser-hosted WASM execution core.

Phase 3 is complete and delivered the runtime-facing package format and development shell with static room viewer.

## Local Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\path\to\game
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample
```

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

## Current Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\path\to\game
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample
```

See `docs/notes/package-format-v1-runtime.md` for the current runtime package contract.
See `docs/superpowers/plans/2026-05-20-opengmk-wasm-first-runtime.md` for the current runtime implementation direction.

## Repository Contents

- `docs/`
  Project documentation and design notes
- `samples/local/iwanna-examples/`
  Local sample corpus used for detector and parser validation
- `vendor/`
  Tracked upstream reference submodules used for GM8 format study and parser research

Planned future areas include:

- `crates/iwm-detector/`
- `crates/iwm-parser/`
- `crates/iwm-cli/`
- later `backend/` and `runtime/` work

## Sample Corpus

The project-local sample corpus is organized under `samples/local/iwanna-examples/`.

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

The Phase 2 package-builder milestone emitted a structural V0 package consisting of:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.json`
- `analysis.json`

That V0 output has been superseded by the current runtime-facing Phase 3 package, which now includes browser-ready `resources/` exports and `scripts.ir.json`.

Important runtime direction note:

- the current package and frontend shell remain useful
- the current TypeScript gameplay runtime should be treated as transitional tooling, not the final compatibility engine
- future runtime-fidelity work should accumulate in the WASM-hosted engine path, not in a parallel TS gameplay reimplementation

Out of scope for the MVP:

- broad support for all Game Maker games
- non-GM engines
- full engine parity from day one

## Notes

- Local sample files should be treated as development assets, not canonical source files
- Do not redistribute copyrighted game binaries casually
- Multi-file packages are not automatically non-targets; many GM8 games ship with DLL, audio, and config files
