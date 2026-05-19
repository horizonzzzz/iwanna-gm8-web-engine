# AGENTS.md

## Project Overview

This repository is an early-stage MVP for a browser-playable IWanna engine targeting mainstream legacy GM8-style fangames.

The intended pipeline is:

1. detect whether an uploaded game package is likely a targetable GM8 fangame
2. parse targetable packages on the backend
3. normalize them into a project-owned package format
4. run that package in a browser runtime

The repository is currently documentation-first. Core implementation crates and tooling are planned, but most code has not been created yet.

## Source Of Truth

Read these files before making structural decisions:

- `README.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/superpowers/plans/2026-05-19-gm8-detector-foundation.md`
- `docs/superpowers/plans/2026-05-19-gm8-parser-and-package-builder.md`
- `samples/README.md`
- `vendor/README.md`

If the repository state and the plan documents diverge, prefer the actual repository contents for what exists, and prefer the spec/plan documents for intended next steps.

## Repository Layout

- `docs/`
  Design specs, implementation plans, and project notes
- `samples/local/iwanna-examples/`
  Local development sample corpus grouped by compatibility risk
- `vendor/`
  Upstream reference repositories used for study and narrow parser integration

Planned future directories:

- `crates/iwm-detector/`
- `crates/iwm-parser/`
- `crates/iwm-cli/`
- `runtime/`
- `backend/`

Do not assume those directories already exist.

## Development Workflow

Current expected development order:

1. execute the detector foundation plan
2. execute the GM8 parser and package builder plan
3. only then start browser runtime work

Avoid skipping ahead to runtime implementation before detector and parser outputs exist.

## Setup Commands

There is currently no bootstrapped Cargo workspace in the repository root yet. The following commands are intended future commands from the implementation plans, not guaranteed working commands in the current state:

- `cargo test`
- `cargo run -p iwm-cli -- detect --input C:\path\to\game`
- `cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\out\sample`

If you are the first agent implementing code here, create the workspace exactly from the implementation plans rather than inventing a different layout.

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

At this stage, testing is expected to be layered:

1. unit tests for detector and parser modules
2. local smoke tests against `samples/local/iwanna-examples/`
3. regression checks against `gm8-core`
4. negative checks against `non-target`

When code exists, always run the narrowest relevant test first, then the broader suite.

## Code Style

Follow these project rules once code is added:

- Rust for detector/parser foundation unless requirements change explicitly
- keep modules small and responsibility-focused
- isolate upstream integration behind adapters
- prefer structured JSON outputs over ad hoc text output
- do not bury engine heuristics inside CLI code
- do not mix detector concerns with runtime concerns

## Pull Request And Change Guidelines

For meaningful changes:

- update docs if the intended architecture or workflow changes
- keep sample-path references aligned with `samples/local/iwanna-examples/`
- note any upstream API drift when changing vendored integration assumptions

If a change invalidates an existing plan document, update the relevant plan or add a note explaining the divergence.

## Debugging Notes

Common likely failure modes for this project:

- false engine classification due to weak string heuristics
- GPL-sensitive dependency decisions made too casually
- path assumptions breaking because samples moved from the desktop into the repo
- trying to implement runtime behavior before stable package outputs exist

When debugging, first determine which layer is actually failing:

- detection
- package loading
- GM8 parsing
- normalization
- runtime

Do not debug all layers at once.
