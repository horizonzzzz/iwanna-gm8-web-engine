# Testing Strategy

> **Current status note:** This document defines the repository's test layering
> and refactor rules for keeping the codebase practical for agent-driven work.

The repository has reached the point where test volume is comparable to source
volume. That is acceptable only when tests are layered clearly. New work should
avoid adding another bespoke end-to-end setup when an existing fixture, contract
test, or scenario runner can express the same behavior.

## Test Layers

Use these layers when adding, moving, or merging tests.

### Unit Tests

Purpose:

- prove small parser, runtime, host, renderer, or formatter behavior
- cover pure value transforms and narrow state transitions
- stay fast enough to run during ordinary edit cycles

Rules:

- prefer table-driven cases for expression lowering, event tag mapping, helper
  functions, and numeric runtime helpers
- avoid constructing a full `RuntimePackage` unless the behavior needs package
  identity, room state, or event dispatch
- keep assertions focused on one behavior or one small state transition

### Contract Tests

Purpose:

- protect boundaries between crates, the upload API, and both browser surfaces
- validate package shape, sparse ids, cross-file references, bridge JSON, and
  host import/export expectations

Rules:

- keep these synthetic and deterministic
- prefer compact fixtures over local sample packages
- validate the public contract, not private implementation details

Current important contract areas:

- `crates/iwm-runtime-model/` package validation
- `crates/iwm-parser/` runtime package output shape
- `crates/iwm-runtime-web/` bridge JSON and host-boundary behavior
- `crates/iwm-api/` upload limits, verdict mapping, validation, and publication
- `runtime/` upload flow, package loading, WASM session input edges, and renderer commands

### Scenario Tests

Purpose:

- prove a visible runtime slice such as jump, room transition, savepoint,
  collision death, file path behavior, or diagnostics grouping
- cover behavior that crosses multiple runtime-core modules

Rules:

- share builders and helpers from the crate test support module
- use named scenario helpers or input scripts instead of repeating setup blocks
- check runtime diagnostics explicitly when the scenario is about unsupported
  coverage or blocker removal

### Local Sample Smoke Tests

Purpose:

- validate the current local gold sample and sample corpus when those files are
  available on the developer machine
- provide evidence for gameplay-priority decisions

Rules:

- never require local copyrighted sample binaries for default repository health
- skip cleanly when `runtime/public/packages/sample/` or the sample corpus is
  absent
- do not promote a local-sample-only assertion into default contract truth
- prefer `iwm-cli runtime-diagnostics` for gold-sample blocker ranking before
  adding new GM helper support

## Default Verification

For narrow changes, run the closest layer first:

```powershell
cargo test -p iwm-runtime-core
cargo test -p iwm-parser
cargo test -p iwm-runtime-web
cargo test -p iwm-api
npm --prefix runtime test
```

For broad parser/runtime/package changes, also run:

```powershell
cargo test
npm --prefix runtime run build
docker build -t iwm-beta:test .
```

Run browser smoke only when local prerequisites are available:

```powershell
npm --prefix runtime run test:browser
```

When code changes affect the repository graph, run:

```powershell
graphify update .
```

## Refactor Rules

- Do not delete coverage simply because a test is long. First classify the test
  layer and identify which contract or scenario it protects.
- Prefer merging tests when multiple cases differ only by input expression,
  event type, expected diagnostic code, or simple fixture field.
- Move repeated setup into test support before splitting implementation modules.
- Keep real sample evidence separate from synthetic fixtures.
- Treat `runtime/public/packages/sample/` as a local artifact, not a stable
  fixture.
- Keep parser/runtime contract changes synchronized with
  `docs/notes/package-format-v1-runtime.md` and
  `docs/notes/runtime-wasm-gap-analysis.md`.

## Near-Term Cleanup Order

1. Keep this test layering document current.
2. Consolidate runtime-core test support around compact package, room, object,
   event, and lowered-logic builders.
3. Split parser package smoke tests into contract-focused files.
4. Convert repeated expression/lowering/runtime helper cases into table-driven
   tests.
5. Split large runtime-core implementation files after tests are easier to read.
6. Keep frontend tests focused on the public upload-to-package handoff and the
   retained shell's bridge, session, renderer, and visible UI contracts.
