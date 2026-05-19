# iwanna-gm8-web-engine

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Overview

This project explores a practical path for running mainstream legacy IWanna fangames in the browser.

The intended pipeline is:

1. accept an original game package
2. detect whether it is likely a supported GM8-style target
3. parse the package on the backend
4. normalize it into a project-owned package format
5. run that package in a browser runtime

The goal is not to emulate every Game Maker game. The first focus is a narrower compatibility subset that can support core IWanna gameplay.

## Current Status

> [!IMPORTANT]
> This repository is still in an early MVP stage. At the moment it mainly contains project documentation, a local sample corpus, and vendored upstream references. The detector, parser, package builder, and browser runtime are planned but not fully bootstrapped yet.

## Repository Contents

- `docs/`
  Project documentation and design notes
- `samples/local/iwanna-examples/`
  Local sample corpus used for detector and parser validation
- `vendor/`
  Upstream reference repositories used for GM8 format study and parser research

Planned future areas include detector, parser, CLI, backend, and runtime code.

## Sample Corpus

The local sample corpus is organized under `samples/local/iwanna-examples/`.

Current categories:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Suggested usage:

- start smoke testing with `gm8-core`
- use `non-target` for negative classification checks
- treat current labels as working development categories, not final truth

## Vendored References

The `vendor/` directory is used for upstream study and narrow integration experiments.

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

Out of scope for the MVP:

- broad support for all Game Maker games
- non-GM engines
- full engine parity from day one

## Notes

- Local sample files should be treated as development assets, not canonical source files
- Do not redistribute copyrighted game binaries casually
- Multi-file packages are not automatically non-targets; many GM8 games ship with DLL, audio, and config files
