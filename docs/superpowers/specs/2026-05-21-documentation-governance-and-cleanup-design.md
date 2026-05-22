# Documentation Governance And Cleanup Design

## Overview

This document defines the current documentation-governance direction for `iwanna-gm8-web-engine`.

The repository should stay aligned through a small set of active specs and notes that describe current reality. Repository-stored implementation plans are intentionally removed because they duplicate or drift away from the docs contributors actually follow.

## Problem Statement

The repository was accumulating two different kinds of guidance:

- current docs that describe the actual parser/runtime direction
- in-repo implementation plans that quickly became stale and started competing with the current docs

That split creates avoidable ambiguity:

1. contributors may follow an older execution sequence instead of the current repo state
2. top-level docs become less trustworthy because they must explain which plans still matter
3. historical planning detail starts to outrank the actual architecture notes by sheer volume

## Goals

- Keep `README.md`, `AGENTS.md`, and the current `docs/notes/*` set trustworthy
- Keep the primary product spec aligned with the current repository shape and package/runtime direction
- Remove repository-stored implementation plans that no longer provide reliable guidance
- Require meaningful repo-state changes to update the relevant current docs in the same change

## Non-Goals

- Preserving every intermediate execution plan in version control
- Replacing current notes with a new planning system
- Rewriting architectural history beyond what current specs and notes need to explain

## Design Principles

### 1. Prefer current truth over stored execution history

If a document competes with `README.md`, the active spec, the current notes, or the actual repository contents, it should be updated, marked superseded, or removed.

### 2. Keep active guidance compact

The repo should not require contributors to decide which of many overlapping planning artifacts is current. A smaller current-doc set is more useful than a larger partially historical one.

### 3. Preserve lasting rationale in specs and notes

If a decision matters long-term, capture it in the product spec or a current note. Do not rely on an old implementation plan remaining nearby forever.

### 4. Documentation updates are part of the change

When implementation changes shift project phase, architecture, package contracts, testing commands, local artifact assumptions, or runtime blockers, the relevant docs should be updated in the same change.

## Active Documentation Set

The current active guidance set should be:

- `README.md`
- `AGENTS.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `samples/README.md`
- `vendor/README.md`

These docs should be enough to answer:

- what the repository currently contains
- what runtime path is active
- what package contract the parser emits
- what sample and verification assumptions are current
- what the next proven blockers are

## Repository Plan Policy

The repository should not keep `docs/superpowers/plans/` as an active documentation tier.

Implementation plans are useful while work is being executed, but once they start drifting from the actual code and notes, they create more confusion than value. Durable decisions should be folded into the current spec or notes instead.

## `AGENTS.md` Expectations

`AGENTS.md` should act as the operator guide for contributors and agents.

It should:

- point to the active source-of-truth docs
- avoid telling contributors to mine old plans for current direction
- require current docs to be updated alongside meaningful repository changes
- make it explicit that implementation plans do not belong in the long-lived repo guidance set

## Cleanup Strategy

Recommended execution order:

1. update `README.md` and `AGENTS.md`
2. update active current-state notes that have drifted from the real repo state
3. update the primary spec where current architecture or repository-shape references are stale
4. remove repository-stored implementation plans
5. run a final repo-wide pass for broken references or stale current-state claims

## Success Criteria

This cleanup is successful when:

- no active doc points contributors at `docs/superpowers/plans/`
- top-level docs and current notes agree on the active parser/runtime direction
- current-state notes do not claim repo-local artifacts exist when they are actually local prerequisites
- contributors can understand the project direction from specs, notes, and code without consulting deleted plans
