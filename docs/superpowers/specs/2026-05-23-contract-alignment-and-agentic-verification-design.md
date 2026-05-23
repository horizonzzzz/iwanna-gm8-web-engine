# Contract Alignment and Agentic Verification Design

## Overview

This document defines the next process-focused design slice for `iwanna-gm8-web-engine`.

The current repository direction is correct, but recent parser/runtime/web regressions show that the project still allows cross-layer contract drift to surface too late. The immediate need is not another gameplay feature. The immediate need is a stronger way to keep the parser-owned package contract, the Rust/WASM runtime contract, and the browser shell contract aligned while development continues to rely heavily on agents.

This design does not change the project runtime mainline. OpenGMK-derived runtime extraction remains the long-term semantic path. The goal here is to reduce avoidable drift and make regressions fail earlier, closer to the layer that introduced them.

## Problem Statement

Recent work exposed three recurring failure modes:

- parser output and runtime consumption can disagree on identity/reference semantics such as `object_id`
- runtime-web bridge JSON and shell TypeScript types can drift when lowered-logic or runtime snapshot shapes change
- local sample smoke checks can hide or overstate regressions because the sample corpus is environment-local and differs across machines

The underlying problem is that the repository currently relies too much on informal agreement between layers:

- package contract described in docs plus ad hoc tests
- lowered-logic contract verified mainly through deserialization and targeted runtime tests
- shell/runtime bridge verified only after a full local browser bring-up

This makes regressions expensive to locate. A drift introduced in parser or runtime often first appears as a browser symptom instead of a contract failure at the seam where the bug actually started.

## Goals

- make parser/runtime/web contract drift fail at the narrowest possible layer
- preserve the current OpenGMK/WASM-first architectural route
- keep `runtime/` as a harness and host adapter, not a second gameplay engine
- make local sample variation explicit so it does not become accidental repository truth
- define an agent-friendly workflow that decomposes cross-layer work into bounded ownership slices

## Non-Goals

- redesign the runtime package format wholesale
- replace the current local sample corpus with tracked copyrighted binaries
- move gameplay semantics back into TypeScript
- block all exploratory work behind heavyweight infrastructure before any progress can continue

## Design Principles

### 1. Treat Contracts As Executable, Not Narrative

The package contract, lowered-logic contract, and bridge contract should be validated by code, not only by docs and human review.

### 2. Fail Earlier Than Browser Smoke

If parser emits a malformed or inconsistent package, that should fail before a room is rendered in the shell.

### 3. Keep Ownership Single-Direction

Parser owns normalized package structure. Runtime owns semantic consumption. Web shell owns host interaction and display. A layer may validate upstream inputs, but it should not compensate for upstream drift with hidden heuristics.

### 4. Prefer Stable Fixtures Over Machine-Local Assumptions

Environment-local samples remain useful for smoke validation, but repository truth should come from synthetic and checked-in non-copyright fixtures.

## Proposed Workstreams

### Workstream 1: Package Contract Validator

Introduce a repository-owned validator for normalized runtime packages.

This validator should check structural and referential integrity, including:

- every `room.instances[*].object_id` resolves to an `objects[*].id`
- every `object.sprite_index` resolves to a sprite resource when non-negative
- every tile/background reference resolves through `resources/index.json`
- every event `block_id` resolves consistently across `scripts.ir.json`, `logic.raw.json`, and `logic.lowered.json` where applicable
- manifest counts match actual payload counts

The validator should be usable in three places:

- directly from tests
- from CLI/package-build verification
- from future CI gates

The validator should report structured, layer-specific failures so agents and humans can identify whether the drift is in parser export, runtime assumptions, or shell expectations.

### Workstream 2: Stable Runtime Fixtures

Add repository-stable fixtures that are safe to commit and do not depend on local proprietary game binaries.

These fixtures should include:

- minimal synthetic packages covering sparse ids, missing resources, event tag normalization, and lowered-logic shapes
- one checked-in gold-style generated package fixture when licensing and asset constraints allow it

Fixture purpose:

- parser-independent runtime regression coverage
- bridge/schema regression coverage
- browser harness smoke coverage against a known package

Local sample packages remain valid for extended smoke testing, but they should not be the only evidence that a contract still works.

### Workstream 3: Verification Gates By Change Type

Define mandatory validation paths based on the layer being changed.

For parser/package changes:

- package validator
- targeted parser tests
- fixture package load checks

For lowered-logic schema changes:

- Rust serde tests
- runtime-core targeted tests
- runtime-web bridge serialization/deserialization tests
- TypeScript fixture load tests if the browser contract changed

For runtime-core/runtime-web changes:

- targeted runtime-core tests
- `iwm-runtime-web` tests
- wasm build and sync verification when bridge-visible behavior changed

For shell-only changes:

- frontend unit/browser tests
- at least one browser smoke against a stable fixture package

The point is not to run every layer on every edit. The point is to stop claiming success based on the wrong verification tier.

### Workstream 4: Agent-Oriented Delivery Workflow

Cross-layer work should be decomposed into owned slices instead of being edited monolithically.

Recommended ownership pattern:

- exploration agent: trace affected contracts, docs, and tests before edits
- parser/package worker: `iwm-parser`, `iwm-runtime-model`, package docs
- runtime worker: `iwm-runtime-core`, `iwm-runtime-web`
- harness/tests/docs worker: shell tests, fixtures, notes, verification docs
- main agent: integration, verification, and final commit

This keeps each agent scoped to one contract boundary and reduces the risk that one agent silently patches around another layer's bug.

## Recommended Delivery Order

The next process-hardening cycle should proceed in this order:

1. formalize the package validator and failure categories
2. add stable sparse-id and missing-reference fixtures
3. wire validator usage into parser/runtime-facing tests
4. add bridge/schema fixture tests for lowered-logic and runtime snapshots
5. document mandatory verification gates for future cross-layer work

This order is deliberate. Validation infrastructure should exist before the next broad parser/runtime push.

## Trade-Offs

### Benefits

- contract drift is caught closer to the source layer
- browser debugging load goes down
- local sample differences stop masquerading as repository regressions
- agent work becomes easier to partition and review

### Costs

- more upfront work on fixtures and validators before visible gameplay features
- additional maintenance for docs and test fixtures
- some duplicate-seeming validation across parser/runtime/web seams

These costs are acceptable because the current cost of late drift discovery is already higher.

## Success Criteria

- a malformed package with unresolved `object_id` or resource references fails in validation before browser smoke
- sparse-id runtime cases are covered by committed tests and fixtures
- lowered-logic schema changes require explicit bridge-level verification before merge
- process guidance exists for agent-scoped cross-layer work and verification gates
- local sample smoke remains useful, but repository correctness no longer depends on a specific machine's sample inventory

## Risks

- validator scope can grow too broad and become a second parser
- fixtures can become stale if package format changes without synchronized updates
- teams may ignore verification gates unless they are wired into default commands or CI

These risks should be mitigated by keeping validator scope strictly structural and contract-oriented, not semantic.

## Constraints

- local copyrighted sample binaries remain untracked
- OpenGMK-derived runtime extraction remains the long-term semantic mainline
- `runtime/` must not regain responsibility for gameplay semantics that belong in Rust runtime layers
