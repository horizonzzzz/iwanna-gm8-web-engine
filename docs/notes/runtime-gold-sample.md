# Runtime Gold Sample

> **Current status note:** This is an active runtime-priority document, not a historical note.
>
> When the primary gold sample, local package availability, or proven blocker list changes, update this file in the same change.

This note keeps both the long-lived validation target and the current blocker audit for the WASM-first path.

Important local-path note:

- these sample paths are local development paths, not tracked sample binaries
- `runtime/public/packages/sample/` is present in this repo state
- that package currently maps to `I wanna be the Dife.exe`, so the repo-local browser smoke path can now use the intended gold sample package directly

## Primary Gold Sample

**Path:** `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

Why it still matters:

- it remains the intended first sample for validating boot, movement, death/reset, and room transition semantics
- it is the sample that should decide whether parser/package/runtime gaps are actually on the critical gameplay path
- it should stay ahead of secondary samples when priorities conflict

## Repo-Local Runtime Package

- `runtime/public/packages/sample/`

This repo-local package is now the primary browser smoke target because it is the checked local package artifact present in the current repo state.

## Blockers By Layer

- Parser missing data: no package-artifact absence currently blocks the gold-sample smoke path; `sample/manifest.json`, `analysis.json`, `scripts.ir.json`, and the exported resources are present.
- Runtime-core semantic gap: the remaining meaningful gaps are the ones that still block movement, collision, death/reset, or room transition after the WASM runtime has already booted and drawn a room.
- Wasm/web host gap: no host-only blocker is currently proven on the checked `sample` browser smoke path; the shell boots `/packages/sample` through the WASM bridge, reports telemetry for `rInit`, and can switch to `rStage01`.
- Shell-only issue: none currently proven on the repo-local `sample` path; the remaining browser-smoke risk is telemetry drift if the shell selectors or sample room IDs change.

## Sample Audit

### IWBT_Dife

- Package path: `runtime/public/packages/sample/`
- Boot room: `2 / rInit`
- Frame draws: verified at shell level; the WASM runtime boots and presents frame telemetry through the browser shell
- Player appears: verified on the current shell path in `rInit`, and also after switching to `147 / rStage01`
- Movement works: not yet verified as a dedicated browser assertion; current smoke proves boot, room switch, and visible player telemetry
- First blocking warning or missing behavior: no boot blocker is currently proven from the repo-local package; the first unresolved runtime gaps are still deeper gameplay semantics behind the successful boot path

Critical-path `runtime-missing-source-lowering:*` warnings:

- no warning is currently proven critical on the checked `rInit` boot path or the verified `rStage01` room-switch smoke path
- do not escalate broad `runtime-missing-source-lowering:*` lists from `analysis.json` as critical until one is tied to spawn, movement, death/reset, or room transition failure on the actual gold-sample path

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
- For the current repo state, `sample/` is the active repo-local smoke package and also the intended `IWBT_Dife` gold-sample package.

## References

- WASM-first runtime plan: `docs/superpowers/plans/2026-05-20-opengmk-wasm-first-runtime.md`
- Package format: `docs/notes/package-format-v1-runtime.md`
- WASM runtime gap analysis: `docs/notes/runtime-wasm-gap-analysis.md`
- Design spec: `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
