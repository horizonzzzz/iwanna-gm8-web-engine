# Local Samples

This directory stores development sample data used to validate detector, parser, API, and runtime behavior.

## Layout

- `local/iwanna-examples/`

Important repository note:

- this repository does not commit the actual sample binaries
- a fresh clone may contain only this README until you add local sample data under `samples/local/`
- scripts and plans may refer to these paths as expected local development locations, not guaranteed tracked files

Current local categories under `local/iwanna-examples/`:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

## Purpose

These samples are used for:

- engine detection validation
- parser smoke testing
- compatibility triage
- runtime regression checks

Important local-environment note:

- `samples/local/iwanna-examples/` is environment-local and may not contain the same files on different development machines
- treat the local corpus as input data for the current machine, not as a single repo-wide fixed inventory

## Current Usage

These samples are used for:

- detector validation
- parser/package smoke testing
- runtime package generation
- runtime regression checks

Important:

- the local sample directories may be absent in a fresh clone
- generated runtime packages under `runtime/public/packages/` are not a substitute for the raw local sample corpus

## Important Notes

- These files are local development assets, not canonical project source
- Do not treat current category labels as permanent truth
- Reclassify samples when stronger parser evidence becomes available
- Be careful with redistribution and copyright

## Regression Workflow

Samples are organized as L1/L2/L3 validation targets. IWBT_Dife is the stable
L1 regression baseline. I Wanna Break Through ArioTrials is the current L2
compatibility-development sample. Larger or extended games are L3 pressure tests
until their earlier pipeline stages are stable.

Run `sample-audit` first to compose detection, package generation, validation,
and bounded runtime diagnostics. Keep generated reports under
`target/sample-audits/` and packages under `runtime/public/packages/`; neither is
a tracked sample artifact.

After a path is stable, use `runtime-scenario`. Scenario JSON files reuse the
existing `ticks` input format and add observable assertions for blockers,
final/visited rooms, instance events, deaths, and final-player numeric ranges.
Scripts make play paths reproducible; Rust tests remain responsible for precise
GM8 semantics and feature-gated real-sample integration.
