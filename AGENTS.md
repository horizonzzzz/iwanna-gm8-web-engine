# AGENTS.md

## Project Overview

This repository is an active MVP for a browser-playable IWanna engine targeting mainstream legacy GM8-style fangames.

The active pipeline is:

1. detect whether an uploaded game package is likely a targetable GM8 fangame
2. parse targetable packages on the backend/tooling side
3. normalize them into a project-owned package format
4. run that package through a browser-facing WASM-first runtime path

The repository is no longer documentation-first. It now contains:

- a Rust workspace
- detector, parser, CLI, and runtime crates
- a browser runtime shell under `runtime/`
- tests covering detector, parser, runtime core, runtime host, runtime web bridge, and frontend shell behavior

## Source Of Truth

Read these files before making structural or workflow decisions:

- `README.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `samples/README.md`
- `vendor/README.md`

Do not treat implementation plans as a source of truth. Keep the repo aligned through current specs, notes, and actual repository contents. Local plan files may be created under `docs/superpowers/plans/` to coordinate work, but they are working artifacts and do not need to be submitted with code changes.

## Repository Layout

- `docs/`
  Design specs, status notes, and project guidance
- `crates/iwm-detector/`
  GM8-target detection and package inventory logic
- `crates/iwm-parser/`
  GM8 parsing, package building, resource export, and logic extraction/lowering
- `crates/iwm-cli/`
  Developer CLI for detection and package building
- `crates/iwm-runtime-model/`
  Shared runtime package schema
- `crates/iwm-runtime-host/`
  Runtime host-boundary types, traits, and headless/default host helpers
- `crates/iwm-runtime-core/`
  Deterministic runtime-core behavior and lowered-logic execution slice
- `crates/iwm-runtime-web/`
  Browser/WASM bridge surface and JSON/FFI bridge helpers
- `runtime/`
  Browser shell, diagnostics UI, package loading, and rendering glue
- `samples/local/iwanna-examples/`
  Local development sample corpus when populated
- `vendor/`
  Upstream reference submodules used for parser/runtime study

Planned later area:

- `backend/`

## Development Workflow

Current expected workflow:

1. keep detector and parser outputs stable enough to generate runtime-facing packages
2. keep runtime work aligned to the parser-owned package contract instead of bypassing it
3. treat the browser shell as a shell/diagnostics harness around the WASM-first runtime path
4. update current notes whenever runtime/package/workflow reality changes

## Setup Commands

Current expected setup commands:

- `git submodule update --init --recursive`
- `cargo test`
- `npm --prefix runtime install`
- `npm --prefix runtime test`

WASM bridge workflow commands:

- `cargo build -p iwm-runtime-web --target wasm32-unknown-unknown`
- `npm --prefix runtime run sync:wasm`

Package generation commands:

- `cargo run -p iwm-cli -- detect --input C:\path\to\game`
- `cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample`
- `cargo run -p iwm-cli -- validate-package --input .\runtime\public\packages\sample`

## Sample Corpus Instructions

Use the local sample corpus under:

- `samples/local/iwanna-examples/`

Current categories:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Rules:

- do not commit copyrighted sample binaries to git
- do not assume category labels are final truth
- prefer `gm8-core` for first smoke tests
- use `non-target` to validate negative classification behavior

## Vendored References

Reference repositories currently present:

- `vendor/OpenGMK/`
- `vendor/GM8Decompiler/`

Use them for:

- studying `gm8exe`
- validating parsing assumptions
- checking edge cases in GM8 executable handling
- auditing runtime semantics and host-boundary assumptions for the WASM-first path

Do not:

- copy large upstream code blocks into project code casually
- couple project-owned modules directly to multiple upstream packages at once

If direct `gm8exe` integration is needed, isolate it behind:

- `crates/iwm-parser/src/gm8_adapter.rs`

## Licensing And Safety

Important:

- `OpenGMK` ecosystem components may be `GPL-2.0-only`
- this is a real architectural constraint, not just a note
- keep integration narrow and intentional

Before expanding from local experimentation to broader distribution, re-check the license implications of any vendored dependency used in builds.

## Testing Instructions

Current testing layers:

1. targeted crate tests for detector, parser, runtime model validator, runtime host, runtime core, or runtime web
2. workspace-wide Rust verification with `cargo test`
3. frontend shell verification with `npm --prefix runtime test`
4. browser smoke verification with `npm --prefix runtime run test:browser` when local prerequisites are satisfied
5. local sample smoke checks against `samples/local/iwanna-examples/` when relevant assets exist

When changing code, run the narrowest relevant test first, then the broader suite.

## Code Style

Follow these project rules:

- Rust remains the default language for detector/parser/runtime foundation unless requirements change explicitly
- keep modules small and responsibility-focused
- isolate upstream integration behind adapters or narrow host-boundary layers
- prefer structured JSON outputs over ad hoc text output
- do not bury engine heuristics inside CLI code
- do not mix detector concerns with runtime concerns

## Pull Request And Change Guidelines

For meaningful changes:

- update docs if the intended architecture or workflow changes
- keep sample-path references aligned with `samples/local/iwanna-examples/`
- note any upstream API drift when changing vendored integration assumptions
- if a change invalidates a current spec or note, update it or mark it clearly as superseded in the same change

If parser or runtime work changes what is actually required for a playable WASM runtime, update `docs/notes/runtime-wasm-gap-analysis.md` in the same change.

## Documentation Maintenance Rules

Documentation is part of the implementation, not follow-up cleanup.

When repository reality changes in a meaningful way, update the relevant docs in the same change. This applies to:

- project phase changes
- architecture direction changes
- package-format or runtime-contract changes
- setup or verification command changes
- important crate/layout changes
- changes to current runtime blockers or gold-sample expectations

Required behavior:

- if an older document is no longer current, mark it clearly as `historical` or `superseded`, or remove it if it no longer provides useful context
- treat `README.md`, `AGENTS.md`, and `docs/notes/runtime-wasm-gap-analysis.md` as high-priority always-current docs
- for parser, runtime, or package-contract changes, check whether `README.md` and the relevant `docs/notes/*` files also need updates
- implementation plan files under `docs/superpowers/plans/` may be used as local working artifacts, but they should not be treated as required commit contents; preserve lasting rationale in current specs or notes instead

## Debugging Notes

Common likely failure modes for this project:

- false engine classification due to weak string heuristics
- GPL-sensitive dependency decisions made too casually
- path assumptions breaking because local samples or generated packages are missing
- parser/runtime contract drift between emitted package data and runtime consumption
- trying to debug detection, parser, normalization, and runtime behavior all at once

When debugging, first determine which layer is actually failing:

- detection
- package loading
- GM8 parsing
- normalization
- runtime

Do not debug all layers at once.
