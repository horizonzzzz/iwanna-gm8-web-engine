# Documentation Governance And Cleanup Design

## Overview

This document defines a repo-wide documentation cleanup pass for the current `iwanna-gm8-web-engine` state.

The goal is not to minimize document count. The goal is to make it obvious which documents describe current reality, which documents are historical planning artifacts, and which documents should be removed because they no longer provide useful context.

This design also establishes explicit documentation maintenance rules in `AGENTS.md` so future implementation work updates documentation as part of normal development rather than as an afterthought.

## Problem Statement

The repository now contains a mix of:

- current fact-bearing documents
- older implementation plans that still have historical value
- outdated instructions that no longer match the actual repository state
- stage-specific notes that have already been superseded

The most severe example is `AGENTS.md`, which still describes the repository as documentation-first and pre-workspace even though the repository now contains a Rust workspace, runtime crates, tests, and a WASM-first runtime direction.

When these documents remain unclassified, they create three problems:

1. agents and contributors may follow stale instructions as if they are current
2. current project status becomes harder to infer from repo docs
3. old documents stay in the tree without any indication of whether they are active guidance or preserved history

## Goals

- Align current high-priority docs with the actual repository state
- Mark preserved-but-outdated docs as `superseded` or `historical`
- Remove low-value outdated docs when they no longer add real context
- Add explicit documentation maintenance rules to `AGENTS.md`
- Keep the repo’s planning history available where it still helps future reasoning

## Non-Goals

- Rewriting every historical plan into a new format
- Deleting all old plans just to reduce file count
- Collapsing all docs into a smaller number of files
- Changing project architecture as part of documentation cleanup

## Design Principles

### 1. Prefer classification over deletion

If an older document still helps explain how the repository got to its current shape, keep it and classify it clearly rather than deleting it reflexively.

### 2. Delete only low-value obsolete docs

A document should be deleted only when:

- its content has effectively been replaced elsewhere
- it no longer helps explain current or historical decisions
- keeping it would create more confusion than value

### 3. Current docs must be trustworthy

Documents that are still likely to be read first by contributors or agents must reflect the actual repository state closely enough that they do not mislead implementation work.

### 4. Historical docs must announce themselves

Older plans and notes can remain in the repo, but they should never look like current instructions if they are no longer the primary source of truth.

### 5. Documentation updates are part of the change

When implementation changes shift project phase, architecture, package contracts, testing commands, or key blockers, the relevant docs should be updated in the same change set.

## Document Tiers

This cleanup should classify documentation into three tiers.

### Tier 1: Current Docs

These documents should be updated to match the current repository state and continue to act as active guidance:

- `AGENTS.md`
- `README.md`
- `samples/README.md`
- `vendor/README.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `docs/notes/sample-corpus.md`

These docs should describe:

- the actual current phase
- actual workspace/crate existence
- current runtime direction
- current testing reality
- current sample and vendor expectations
- current documentation-update obligations

### Tier 2: Historical Or Superseded Docs

These documents should be retained but explicitly marked as no longer primary implementation guidance when they no longer reflect the current route:

- `docs/superpowers/plans/2026-05-19-gm8-detector-foundation.md`
- `docs/superpowers/plans/2026-05-19-gm8-parser-and-package-builder.md`
- `docs/superpowers/plans/2026-05-19-runtime-shell-and-static-room-viewer.md`
- `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md`
- `docs/superpowers/plans/2026-05-20-wasm-browser-input-render-loop.md`
- any other plan that remains useful for historical context but is no longer the primary path

The intended status language should be explicit, for example:

- `Status note: historical`
- `Status note: superseded`
- `Superseded by: <newer doc>`
- `Use for historical context only; do not use as the primary implementation source`

These labels should be placed near the top so contributors do not need to read the full document to discover that it is outdated.

### Tier 3: Delete Candidates

These documents can be removed when their continued presence no longer provides useful context.

Current recommended delete candidate:

- `docs/notes/package-format-v0.md`

Reasoning:

- the repo has clearly moved beyond this format
- the current package contract is already documented in `docs/notes/package-format-v1-runtime.md`
- V0 is a narrow intermediate artifact and does not appear to be a high-value historical reference compared to the plans/specs that already describe the earlier phase

If deleted, the current package-format documentation should still make it clear that V0 is obsolete or superseded.

## `AGENTS.md` Changes

`AGENTS.md` should be treated as an active operator guide, not a historical artifact.

The cleanup should update it in three ways.

### 1. Fix Current-State Mismatches

Remove or rewrite stale claims such as:

- the repo being documentation-first with little code
- the root Cargo workspace not existing
- runtime directories being only planned
- setup commands being only future-intended commands

### 2. Update Source-Of-Truth Guidance

The source-of-truth section should continue pointing to the most useful current docs, but it should avoid implying that old plan documents are still the main operational guidance when README and current notes are more authoritative.

### 3. Add Explicit Documentation Maintenance Rules

`AGENTS.md` should explicitly require that contributors update docs whenever repository reality changes in ways that affect development understanding.

Required rule content:

- if project phase, architecture direction, package contract, key runtime blockers, setup commands, or important repo layout changes, update the relevant docs in the same change
- if an older doc is no longer current, mark it `superseded`/`historical` or remove it
- `README.md`, `AGENTS.md`, and `docs/notes/runtime-wasm-gap-analysis.md` should be treated as high-priority frequently-synchronized docs
- runtime, parser, and package-contract changes should trigger a check for matching note updates

## Historical Plan Handling

Older plan documents should not all be deleted. Instead:

- plans that explain major route changes should stay
- plans that represent superseded runtime directions should be labeled clearly
- plans that still match current direction can remain unlabeled if they are still current enough to be useful

This keeps architectural history available without confusing it with current execution guidance.

## Cleanup Strategy

Recommended execution order:

1. update `AGENTS.md` and `README.md`
2. update active `docs/notes/*` documents that are clearly current-state references
3. add status labels to stale or historical plan docs
4. delete only the low-value obsolete documents
5. run a final repo-wide pass to ensure no remaining doc strongly contradicts current repo reality without a status warning

This order minimizes the period where top-level docs still point contributors at stale instructions.

## Success Criteria

This cleanup is successful when:

- a contributor reading `AGENTS.md` or `README.md` gets an accurate picture of current repository state
- historical plans are visibly labeled as historical or superseded where appropriate
- truly obsolete low-value docs are removed
- the repo contains an explicit rule that documentation must be updated alongside meaningful architectural or workflow changes
- no obviously stale doc remains unlabeled while still appearing current

## Risks

### 1. Over-deleting historical context

If too many old documents are removed, future contributors lose visibility into why the repo took its current shape. This is why classification is preferred over deletion.

### 2. Under-classifying stale docs

If old plans remain in place without prominent status labels, the cleanup fails its main purpose even if current docs are updated.

### 3. Turning cleanup into broad rewriting

Trying to rewrite every historical document in detail would create unnecessary churn. The goal is to classify and correct, not to re-author the repo’s full history.

## Expected Outcome

After this pass:

- current docs will be trustworthy enough for day-to-day development
- outdated docs will remain available without pretending to be current
- clearly obsolete low-value docs will be removed
- `AGENTS.md` will require documentation maintenance as an explicit development responsibility
