# Runtime Gold Sample

> **Current status note:** This is an active runtime-priority document, not a historical note.
>
> When the primary gold sample, local package availability, or proven blocker list changes, update this file in the same change.

This note keeps both the long-lived validation target and the current blocker audit for the WASM-first path.

Important local-path note:

- these sample paths are local development paths, not tracked sample binaries
- `runtime/public/packages/sample/` is not present in this repo state
- `IWBT_Dife` therefore remains the intended gold sample, but only as a placeholder until a local `sample/` package is built

## Primary Gold Sample

**Path:** `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

Why it still matters:

- it remains the intended first sample for validating boot, movement, death/reset, and room transition semantics
- it is the sample that should decide whether parser/package/runtime gaps are actually on the critical gameplay path
- it should stay ahead of secondary samples when priorities conflict

## Comparison Samples Available In This Repo

- `runtime/public/packages/kamilia/`
- `runtime/public/packages/mashikaku/`

These repo-local packages are useful for smoke checks while `runtime/public/packages/sample/` is still missing.

## Blockers By Layer

- Parser missing data: `runtime/public/packages/sample/analysis.json` and `runtime/public/packages/sample/scripts.ir.json` do not exist yet, so the gold sample cannot be audited from a real package artifact.
- Runtime-core semantic gap: the remaining meaningful gaps are the ones that still block movement, collision, death/reset, or room transition after the WASM runtime has already booted and drawn a room.
- Wasm/web host gap: no host-only blocker is currently proven from the checked local packages; both `kamilia` and `mashikaku` boot through the WASM bridge.
- Shell-only issue: the browser shell cannot smoke-test `/packages/sample` because the package directory is absent in the current repo state.

## Sample Audit

### IWBT_Dife

- Package path: `runtime/public/packages/sample/` (missing)
- Boot room: unavailable until `sample/` exists
- Frame draws: not verified
- Player appears: not verified
- Movement works: not verified
- First blocking warning or missing behavior: package artifacts are absent; keep this sample as the primary placeholder until `sample/` is generated

Critical-path `runtime-missing-source-lowering:*` warnings:

- not classifiable yet because the package artifact is missing

### Kamilia

- Package path: `runtime/public/packages/kamilia/`
- Boot room: `0 / startRoom`
- Frame draws: verified at shell level; the WASM runtime booted `startRoom` and requested room sprite resources including `sprites/2-0.png`
- Player appears: not separately proven from current browser evidence
- Movement works: not yet verified; the current smoke only proves tick advancement
- First blocking warning or missing behavior: no boot blocker is currently proven, but the first unresolved boot-path warning is `runtime-missing-source-lowering:room:0:create`

Critical-path `runtime-missing-source-lowering:*` warnings:

- proven boot-path warning: `room:0:create`
- early-path warning that is still unverified as a blocker: `room:1:create`

### Mashikaku

- Package path: `runtime/public/packages/mashikaku/`
- Boot room: `2 / rInit`
- Frame draws: verified on the current playable smoke path; the WASM runtime boots `rInit`, and `room:87 / rStage01` draws with real background and sprite requests
- Player appears: verified on `room:87 / rStage01` on the current smoke path
- Movement works: not yet verified as a dedicated browser assertion
- First blocking warning or missing behavior: no boot blocker is proven from current artifacts; the first remaining missing behavior is movement/room-transition fidelity, not room loading

Critical-path `runtime-missing-source-lowering:*` warnings:

- no warning is currently proven critical on the checked boot path or the verified `room:87` smoke path
- do not escalate `room:87:instance:138622:create`, `138623:create`, `138624:create`, or `138909:create` as critical yet, because `room:87` already draws and shows the player through the WASM path

## Current Validation Behaviors To Prove

For `IWBT_Dife`, Phase 4 still needs to prove:

- package loads successfully
- the runtime core boots the intended first playable room
- the browser host can drive deterministic ticks
- player movement and collision match the runtime-core semantic slice being implemented
- at least one room transition works through the WASM path
- diagnostics stay explicit when unsupported logic, externals, or host gaps are hit

## Notes

- `runtime-missing-source-lowering:*` warnings are not uniformly urgent; only warnings proven to sit on the first-room, spawn, movement, death/reset, or transition path should drive immediate runtime work.
- For the current repo state, `kamilia` and `mashikaku` are smoke references, not replacements for the intended `IWBT_Dife` gold sample.

## References

- WASM-first runtime plan: `docs/superpowers/plans/2026-05-20-opengmk-wasm-first-runtime.md`
- Package format: `docs/notes/package-format-v1-runtime.md`
- WASM runtime gap analysis: `docs/notes/runtime-wasm-gap-analysis.md`
- Design spec: `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
