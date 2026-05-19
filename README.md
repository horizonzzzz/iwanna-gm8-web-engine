# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## What This Project Is

This project explores a practical path to running mainstream legacy IWanna fangames in the browser without trying to emulate every Game Maker title or every fangame engine.

The working strategy is:

1. accept original game packages
2. detect whether they are likely GM8-style fangames
3. parse targetable games on the backend
4. normalize them into a project-owned package format
5. execute that package in a browser runtime

The repository is still at a planning-and-bootstrap stage. The design and implementation plans are already written, but most code has not been created yet.

## Current Scope

- detect likely GM8-style fangame packages
- parse targetable GM8 executables
- normalize parsed structure into a project-owned package format
- later execute that package in a browser runtime

## Current Status

> [!IMPORTANT]
> This repository is not fully bootstrapped yet. The current root contains design docs, implementation plans, local sample data, and vendored upstream references. The planned Rust workspace and runtime code are the next execution steps.

## Local Reference Assets

- sample corpus: `samples/local/iwanna-examples/`
- vendored upstream references: `vendor/`

See:

- `samples/README.md`
- `vendor/README.md`
- `docs/notes/sample-corpus.md`

## Planning Docs

- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/superpowers/plans/2026-05-19-gm8-detector-foundation.md`
- `docs/superpowers/plans/2026-05-19-gm8-parser-and-package-builder.md`

## Repository Structure

- `docs/`
  Design spec, phased implementation plans, and project notes
- `samples/`
  Local IWanna sample corpus for detector/parser/runtime validation
- `vendor/`
  Upstream reference repositories such as `OpenGMK` and `GM8Decompiler`

Planned future directories include detector, parser, CLI, backend, and runtime code. They do not all exist yet.

## Local Workflow

Recommended execution order:

1. build the detector foundation
2. build the GM8 parser and package builder
3. only then begin browser runtime work

## Commands

These are intended target commands from the current implementation plans. They may not work until the first planned code phase is executed:

```bash
cargo test
cargo run -p iwm-cli -- detect --input C:\path\to\game
```

Expected future package-builder command:

```bash
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\out\sample
```

## Sample Corpus

The local sample corpus is organized by current compatibility expectation:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Use `gm8-core` for first smoke tests and `non-target` to validate negative detection behavior.

## Upstream References

The `vendor/` directory currently contains:

- `OpenGMK`
- `GM8Decompiler`

These are included for study and narrow integration, especially around `gm8exe`.

> [!CAUTION]
> Parts of the OpenGMK ecosystem use `GPL-2.0-only`. Treat any direct code reuse or compiled dependency integration as a deliberate architectural decision.

## Next Step

If you are continuing development in a new session, start by reading:

1. `AGENTS.md`
2. the design spec
3. the detector foundation plan

Then execute the first implementation plan rather than inventing a new repository layout.
