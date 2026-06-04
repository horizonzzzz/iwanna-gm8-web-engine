# Runtime Gold Sample

> **Current status note:** This is an active runtime-priority document, not a historical note.
>
> When the primary gold sample, local package availability, or proven blocker list changes, update this file in the same change.

This note keeps both the long-lived validation target and the current blocker audit for the WASM-first path.

Important local-path note:

- these sample paths are local development paths, not tracked sample binaries
- `runtime/public/packages/sample/` is the intended local output path for a generated smoke package, not a tracked artifact
- a fresh clone keeps `runtime/public/packages/` empty except for `.gitkeep`

## Primary Gold Sample

**Path:** `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

Why it still matters:

- it remains the intended first sample for validating boot, movement, death/reset, and room transition semantics
- it is the sample that should decide whether parser/package/runtime gaps are actually on the critical gameplay path
- it should stay ahead of secondary samples when priorities conflict

Important local-environment note:

- the local sample corpus may differ between development machines
- if this exact path is absent in the current environment, use the closest available `gm8-core` sample for local package-smoke verification
- that local fallback does not change the repository's intended gold-sample target or repo-wide contract

## Repo-Local Runtime Package

- `runtime/public/packages/sample/`

This is the intended local browser smoke target after generating a package from the gold sample. The repo does not currently ship a checked package artifact there.

## Blockers By Layer

- Parser/package availability: a fresh clone does not include `runtime/public/packages/sample/`, so the first gold-sample smoke prerequisite is still a local `build-package` run against `IWBT_Dife`.
- Runtime-core semantic gap: the remaining meaningful gaps are the ones that still block movement, collision, death/reset, or room transition after the WASM runtime has already booted and drawn a room.
- Wasm/web host gap: the host telemetry path exists, but gold-sample-specific runtime claims still require evidence from a locally generated `sample/` package.
- Shell-only issue: the default package path remains `/packages/sample`, so missing local artifacts should surface as explicit load errors rather than being mistaken for runtime-semantic failures.

Important validation note:

- because local sample inventories differ across machines, local gold-sample smoke should be treated as environment evidence, not as the only repository-level proof that a parser/runtime/package contract still holds
- stable repository fixtures and package-contract validation should catch structural drift before gold-sample browser debugging is needed
- the current runtime slice already covers alarm dispatch, held/press/release keyboard dispatch, and parent-aware event lookup, so the next gold-sample blockers should be judged against the remaining runtime gap rather than those already-covered slices
- jump is no longer a fixed-height placeholder in repository fixtures; the remaining gold-sample jump work is numeric calibration of tap, hold, release-cut, and landing-reset behavior against `IWBT_Dife`
- the shell/runtime snapshot path now exposes grounded plus jump-phase trace flags for the player, which makes browser-side hand-feel debugging easier but does not change the remaining semantic blocker: the gold sample still needs its own player movement path executed accurately

## Sample Audit

### IWBT_Dife

- Intended package path: `runtime/public/packages/sample/`
- Boot room: not repo-proven without a local generated package artifact
- Frame draws: the shell and WASM bridge can render telemetry, but gold-sample-specific frame proof still depends on local package generation
- Player appears: not repo-proven on a tracked artifact
- Movement works: not yet verified as a dedicated browser assertion
- First blocking warning or missing behavior: not yet narrowed on a tracked gold-sample artifact; use `docs/notes/runtime-wasm-gap-analysis.md` plus local sample evidence to decide the next blocker
- Sprite collision metadata is now present in the parser-emitted package contract as aggregated bbox bounds plus gm8exe-derived `collision_masks`; current runtime pixel checks use the first available sprite mask, while animated per-frame mask selection remains deferred

Critical-path `runtime-missing-source-lowering:*` warnings:

- do not escalate broad `runtime-missing-source-lowering:*` lists from `analysis.json` as critical until one is tied to spawn, movement, death/reset, or room transition failure on the actual locally generated gold-sample path

## Current Validation Behaviors To Prove

For `IWBT_Dife`, Phase 4 still needs to prove:

- package loads successfully
- the runtime core boots the intended first playable room
- the browser host can drive a stable 60 Hz auto-tick loop with pause/resume control for hand-feel validation
- player movement and collision match the runtime-core semantic slice being implemented
- variable-height jump, release cut, and landing reset match the intended `IWBT_Dife` trajectory closely enough to use as the runtime jump baseline
- broader collision dispatch and remaining room/lifecycle semantics still behave as expected now that keyboard and alarm slices are covered
- at least one room transition works through the WASM path
- diagnostics stay explicit when unsupported logic, externals, or host gaps are hit

## Notes

- `runtime-missing-source-lowering:*` warnings are not uniformly urgent; only warnings proven to sit on the first-room, spawn, movement, death/reset, or transition path should drive immediate runtime work.
- Do not treat the absence of `runtime/public/packages/sample/` in a fresh clone as evidence of a runtime failure; it is a local artifact prerequisite.

## References

- Package format: `docs/notes/package-format-v1-runtime.md`
- WASM runtime gap analysis: `docs/notes/runtime-wasm-gap-analysis.md`
- Runtime vendor reference map: `docs/notes/runtime-vendor-reference-map.md`
- Design spec: `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
