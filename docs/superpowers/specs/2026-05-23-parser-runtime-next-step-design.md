# Parser and Runtime Next-Step Design

## Overview

This document defines the next implementation slice for `iwanna-gm8-web-engine`.

The current codebase has already switched to a WASM-first runtime direction, and the next step is to make that direction reproducible on the gold sample while tightening the parser-owned runtime contract. The goal is not to expand game scope or finish OpenGMK extraction in one pass. The goal is to harden the current parser/runtime seam so the repository can keep proving progress on a real IWanna sample without widening the shell-side gameplay surface.

## Current Baseline

The current baseline is:

- `iwm-parser` passes its current test suite.
- `iwm-runtime-core` passes its current test suite.
- `iwm-runtime-web` bridge tests pass.
- `runtime/public/packages/sample/` is still a local artifact path, not a tracked repository asset.
- the `local_package_smoke` test only becomes meaningful after a local `build-package` run produces `logic.lowered.json` and the other runtime package files.
- the exact local sample used to generate that package may differ between development machines, but the artifact contract does not.

This means the next work is not "make the browser shell smarter". The next work is "make the parser/runtime contract stronger on the critical path, then verify it against the local gold sample".

## Scope

### In Scope

- gold sample package generation and smoke verification
- parser lowering on the IWBT_Dife critical path
- runtime-core consumption of lowered logic and host-edge input
- web bridge behavior as a thin JSON/FFI adapter
- current runtime notes and gold-sample notes if behavior or assumptions change

### Out Of Scope

- full OpenGMK extraction
- audio playback
- animation parity
- broader lifecycle parity beyond the current critical path
- package format redesign
- adding new browser gameplay rules in `runtime/`

## Goals

- The gold sample can be regenerated locally into `runtime/public/packages/sample/`.
- The parser keeps structured callable, member, index, and control-flow information for the runtime-critical path.
- The runtime core keeps consuming the normalized package and lowered logic contract without pushing gameplay logic back into the browser shell.
- The web bridge stays thin and deterministic.
- Current docs continue to match actual runtime behavior and local artifact assumptions.

## Proposed Approach

Execute the next step in four ordered workstreams:

1. make the local gold-sample package contract explicit and reproducible
2. harden parser lowering where the gold sample still falls back to raw source
3. keep runtime-core focused on lowered logic, event dispatch, and host-edge input
4. keep the web bridge and docs aligned with the package contract

The parser remains the source of structured runtime data. The runtime core consumes that structure. The browser shell and bridge remain inspection and host glue, not a gameplay rules layer.

## Risks

- local artifacts can be mistaken for repository regressions
- overfitting to a single gold sample can hide general parser gaps
- raw fallback can quietly become a de facto contract if the parser is not tightened
- docs can drift if package assumptions change without a note update

## Success Criteria

- the gold sample package can be built locally and loaded by the bridge smoke path
- parser tests continue to pass after lowering changes
- runtime-core tests continue to pass after host-edge and event-path changes
- web bridge tests continue to pass
- current notes still describe the active runtime/package contract accurately

## Implementation Constraint

This slice should stay focused on parser/runtime contract hardening. If a change would widen supported gameplay semantics beyond the current critical path, it should be treated as a separate follow-up.
