# Sample Regression Workflow Design

## Status

Approved for implementation on 2026-07-12.

## Objective

Turn local GM8 samples into a repeatable compatibility-development workflow without committing copyrighted binaries or generated packages. Keep IWBT_Dife as the stable L1 regression baseline and introduce I Wanna Break Through ArioTrials as the first L2 development sample.

The workflow must support two distinct activities:

1. sample audit: determine which pipeline layer first blocks a sample;
2. runtime scenario: replay a deterministic input sequence and verify stable outcomes.

## Scope

This change will:

- add a CLI sample-audit workflow that composes detection, package generation, package validation, and a bounded runtime diagnostic run;
- emit a structured JSON audit report suitable for comparison and documentation;
- add a reusable runtime-scenario runner that loads an input script, runs the package, evaluates declarative assertions, and returns a failing exit status when assertions fail;
- retain existing feature-gated real-Dife Rust regressions;
- add Dife scenario examples for known stable behavior;
- audit the local ArioTrials sample and add initial scenarios for the stable path that the current runtime can actually reach;
- document how local audits, scenario runs, synthetic tests, real-sample tests, and browser smoke tests fit together.

This change will not:

- commit sample executables, copyrighted assets, or generated runtime packages;
- attempt to fix every compatibility issue discovered in ArioTrials;
- replace narrow synthetic tests for individual GM8 semantics;
- make locally unavailable samples mandatory for the default CI suite.

## Architecture

### Sample audit

`iwm-cli sample-audit` will accept a source sample path, a package output path, a bounded tick count, and an audit-report output path. It will orchestrate existing project-owned detector, parser/package builder, validator, and runtime diagnostics functionality rather than invoking nested CLI processes.

The audit report will record, where available:

- source and generated-package paths;
- detection verdict and evidence summary;
- package build and validation status;
- resource, room, object, and lowered-logic counts;
- manifest default room;
- runtime rooms visited during the bounded run;
- grouped runtime blockers and the first blocking diagnostic;
- external-file or extension-related evidence already exposed by package analysis;
- final success, partial, failed, or skipped status per stage.

A missing local source sample is an explicit skipped result when used through test helpers, but a direct CLI invocation with a nonexistent path remains an error. This prevents CI from silently claiming sample coverage while allowing feature-gated local tests to skip predictably.

### Runtime scenarios

Scenario files remain JSON documents under `docs/notes/runtime-scenarios/`. Their input-event section will remain compatible with the existing runtime-diagnostics input-script format. An optional assertion section will describe stable observable outcomes.

The initial assertion vocabulary will remain intentionally small:

- no runtime blockers;
- final room equals a specified room;
- a room was visited;
- an instance/object create or destroy event occurred, optionally with a minimum count;
- a death event occurred, optionally for a specified object or reason;
- final player state exists and optional numeric fields fall within inclusive ranges;
- named diagnostics or runtime events have an expected minimum count.

Assertions compare observable runtime output rather than internal implementation details. Exact tick assertions will be used only where the behavior is known to be stable; ranges and minimum counts are preferred for movement and particle-heavy behavior.

`iwm-cli runtime-scenario` will accept the same package selection, warmup, room selection, tick, and trace controls needed by runtime diagnostics. It will emit a structured result containing the diagnostic output plus an assertion summary. Any failed assertion produces a nonzero exit code.

### Shared execution layer

Audit and scenario commands will reuse library-level execution helpers extracted from the existing CLI command implementation. CLI argument parsing, output formatting, and exit-code policy remain in `iwm-cli`; detector, parser, validator, runtime model, and runtime core responsibilities remain in their existing crates.

No detector heuristic or runtime semantic rule will be embedded in CLI orchestration code.

## Sample Strategy

### L1: IWBT_Dife

Dife remains the compatibility safety net. Existing real-sample tests continue to cover movement, shooting, death feedback, savepoint behavior, save/load, views, and related runtime semantics. Declarative scenario files will demonstrate the new runner on a small stable subset without duplicating every Rust regression.

At minimum, the new scenario workflow should cover:

- held jump in room 143 with no blockers and bounded player movement;
- shooting in room 143 with bullet lifecycle evidence;
- rightward hazard death in room 151 with a death event and feedback creation.

### L2: ArioTrials

The local path is `samples/local/iwanna-examples/gm8-core/I Wanna Break Through ArioTrials`.

The first pass will run the audit workflow and identify the deepest stable reachable point. Scenarios will be added only for behavior confirmed by the audit or manual comparison. Expected initial candidates are package boot, menu navigation, first gameplay-room entry, and a short no-input or basic-movement window.

ArioTrials failures discovered during this pass will be recorded as follow-up compatibility work. They are not automatically in scope for this implementation unless a small correction is required to make the audit/scenario infrastructure itself valid.

## Testing

The implementation will use four layers:

1. ordinary Rust tests with synthetic fixtures for audit serialization, scenario parsing, assertion evaluation, and exit-result behavior;
2. existing feature-gated local Dife tests for real parser/runtime integration;
3. local CLI runs against Dife and ArioTrials packages, skipped or clearly unavailable when local samples are absent;
4. existing frontend tests and a release-WASM browser smoke when the CLI/runtime changes affect the browser-facing package or runtime contract.

The narrowest relevant tests run first, followed by workspace `cargo test` and `npm --prefix runtime test`. Release WASM rebuild and browser smoke are required only if implementation changes runtime-web output or browser behavior; pure CLI orchestration does not require rebuilding the shipped WASM artifact.

## Documentation

The implementation will update:

- `docs/notes/runtime-gold-sample.md` to define Dife as L1 and record ArioTrials as the current L2 development sample;
- `docs/notes/runtime-wasm-gap-analysis.md` if the audit exposes or changes current runtime blockers;
- `samples/README.md` with the audit/scenario workflow and local-sample rules;
- CLI usage documentation where current commands are listed.

Audit outputs produced from local copyrighted samples will default to `target/` and will not be committed. Stable scenario definitions and non-copyrighted expected assertions may be committed.

## Completion Criteria

The work is complete when:

- the audit command can produce a structured report from a valid local sample and identifies stage-specific failures;
- the scenario command can run existing input scripts plus declarative assertions and fail reliably on a broken expectation;
- synthetic tests cover the new data contracts and assertion evaluator;
- Dife scenarios pass against the current locally generated package;
- ArioTrials has a saved local audit result and at least one repeatable scenario if the current runtime reaches a stable point;
- missing local samples do not break the default test suite or create false passing coverage;
- current project notes describe the resulting workflow accurately.
