# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Phase

Phase 1 adds a Rust workspace and a detector that classifies game packages before parser or runtime work begins.

## Local Commands

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\path\to\game
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

- Phase 1: add a Rust workspace plus detector and CLI
- Phase 2: add a GM8 parser adapter and emit a V0 normalized package
- Phase 3: build a browser runtime against a stricter runtime-facing package format

## Repository Contents

- `docs/`
  Project documentation and design notes
- `samples/local/iwanna-examples/`
  Local sample corpus used for detector and parser validation
- `vendor/`
  Notes and local-only upstream reference checkouts used for GM8 format study and parser research

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
- heavyweight upstream checkouts under `vendor/` are intended to stay local and git-ignored
- path dependencies such as `vendor/OpenGMK/gm8exe` should be treated as local development prerequisites, not guaranteed tracked files

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
- preparing for a browser runtime that can execute core gameplay

The first package-builder milestone is expected to emit a structural V0 package consisting of:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.json`
- `analysis.json`

This V0 output is intentionally not the final runtime-facing package. Script IR lowering and browser-ready resource export come later.

Out of scope for the MVP:

- broad support for all Game Maker games
- non-GM engines
- full engine parity from day one

## Notes

- Local sample files should be treated as development assets, not canonical source files
- Do not redistribute copyrighted game binaries casually
- Multi-file packages are not automatically non-targets; many GM8 games ship with DLL, audio, and config files
