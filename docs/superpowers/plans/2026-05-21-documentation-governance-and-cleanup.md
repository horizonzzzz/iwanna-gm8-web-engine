# Documentation Governance And Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align current repository documentation with actual project state, explicitly classify historical plans, remove low-value obsolete docs, and add durable documentation-maintenance rules to `AGENTS.md`.

**Architecture:** Treat the documentation tree as a governed system with three tiers: current docs, historical/superseded docs, and delete candidates. Update top-level operator docs first, then active notes, then historical plan status banners, and finally remove low-value obsolete docs. Keep historical context where it still explains the current repository shape.

**Tech Stack:** Markdown documentation, repository planning docs, Rust/runtime project context, git history-aware cleanup

---

## File Structure

Planned files for this phase:

- Modify: `AGENTS.md`
- Modify: `README.md`
- Modify: `samples/README.md`
- Modify: `vendor/README.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`
- Modify: `docs/notes/runtime-gold-sample.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/opengmk-host-coupling-audit.md`
- Modify: `docs/notes/sample-corpus.md`
- Modify: `docs/superpowers/plans/2026-05-19-gm8-detector-foundation.md`
- Modify: `docs/superpowers/plans/2026-05-19-gm8-parser-and-package-builder.md`
- Modify: `docs/superpowers/plans/2026-05-19-runtime-shell-and-static-room-viewer.md`
- Modify: `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md`
- Modify: `docs/superpowers/plans/2026-05-20-wasm-browser-input-render-loop.md`
- Delete: `docs/notes/package-format-v0.md`

Responsibilities:

- `AGENTS.md`: active contributor/operator guide and documentation-maintenance policy
- `README.md`: top-level current-state summary
- `docs/notes/*`: active factual notes that should reflect current runtime/package/workflow reality
- `docs/superpowers/plans/*`: preserved planning history with clear status labeling when no longer primary guidance
- `docs/notes/package-format-v0.md`: low-value obsolete intermediate note to remove

### Task 1: Audit The Current Documentation Baseline

**Files:**
- Modify: none

- [ ] **Step 1: Record the current branch doc status before edits**

Run:

```powershell
git status --short
```

Expected:

```text
working tree is clean or only contains the in-progress documentation-governance plan/spec files
```

- [ ] **Step 2: Grep for known stale claims that should disappear or be rewritten**

Run:

```powershell
rg "documentation-first|most code has not been created|do not assume those directories already exist|no bootstrapped Cargo workspace|future commands|TS-first|project-owned TypeScript runtime" AGENTS.md README.md docs
```

Expected:

```text
matches identify the outdated claims that this cleanup must remove, rewrite, or classify
```

- [ ] **Step 3: List the early runtime plans that need status banners**

Run:

```powershell
Get-ChildItem docs/superpowers/plans | Select-Object Name
```

Expected:

```text
includes the 2026-05-19 and 2026-05-20 runtime direction plans that should be marked historical or superseded
```

### Task 2: Rewrite `AGENTS.md` As A Current Operator Guide

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Replace the stale project-overview and repository-state claims**

Rewrite the opening sections of `AGENTS.md` so they no longer claim the repo is documentation-first or pre-implementation.

Use this content for the opening sections:

```md
# AGENTS.md

## Project Overview

This repository is an active MVP for a browser-playable IWanna engine targeting mainstream legacy GM8-style fangames.

The active pipeline is:

1. detect whether an uploaded game package is likely a targetable GM8 fangame
2. parse targetable packages on the backend/tooling side
3. normalize them into a project-owned package format
4. run that package through a browser-facing WASM-first runtime path

The repository is no longer documentation-first. It now contains:

- a Rust workspace
- detector, parser, CLI, and runtime crates
- a browser runtime shell under `runtime/`
- tests covering detector, parser, runtime core, runtime host, runtime web bridge, and frontend shell behavior
```

- [ ] **Step 2: Replace the source-of-truth section with current guidance**

Use this source-of-truth section:

```md
## Source Of Truth

Read these files before making structural or workflow decisions:

- `README.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `samples/README.md`
- `vendor/README.md`

Use plan documents in `docs/superpowers/plans/` carefully:

- some plans remain current enough to mine for execution details
- some plans are historical or superseded and should say so near the top
- if a plan conflicts with `README.md`, current notes, or actual repository contents, prefer the current repository state and current-note documents
```

- [ ] **Step 3: Rewrite the repository-layout and workflow sections**

Use this content:

```md
## Repository Layout

- `docs/`
  Design specs, implementation plans, status notes, and project guidance
- `crates/iwm-detector/`
  GM8-target detection and package inventory logic
- `crates/iwm-parser/`
  GM8 parsing, package building, resource export, and logic extraction/lowering
- `crates/iwm-cli/`
  Developer CLI for detection and package building
- `crates/iwm-runtime-model/`
  Shared runtime package schema
- `crates/iwm-runtime-host/`
  Runtime host-boundary types, traits, and headless/default host helpers
- `crates/iwm-runtime-core/`
  Deterministic runtime-core behavior and lowered-logic execution slice
- `crates/iwm-runtime-web/`
  Browser/WASM bridge surface and JSON/FFI bridge helpers
- `runtime/`
  Browser shell, diagnostics UI, package loading, and rendering glue
- `samples/local/iwanna-examples/`
  Local development sample corpus when populated
- `vendor/`
  Upstream reference submodules used for parser/runtime study

Planned later area:

- `backend/`

## Development Workflow

Current expected workflow:

1. keep detector and parser outputs stable enough to generate runtime-facing packages
2. keep runtime work aligned to the parser-owned package contract instead of bypassing it
3. treat the browser shell as a shell/diagnostics harness around the WASM-first runtime path
4. update current notes whenever runtime/package/workflow reality changes
```

- [ ] **Step 4: Replace the setup and testing sections with current commands**

Use this content:

```md
## Setup Commands

Current expected setup commands:

- `git submodule update --init --recursive`
- `cargo test`
- `npm --prefix runtime install`
- `npm --prefix runtime test`

WASM bridge workflow commands:

- `cargo build -p iwm-runtime-web --target wasm32-unknown-unknown`
- `npm --prefix runtime run sync:wasm`

Package generation commands:

- `cargo run -p iwm-cli -- detect --input C:\path\to\game`
- `cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample`

## Testing Instructions

Current testing layers:

1. targeted crate tests for detector, parser, runtime host, runtime core, or runtime web
2. workspace-wide Rust verification with `cargo test`
3. frontend shell verification with `npm --prefix runtime test`
4. browser smoke verification with `npm --prefix runtime run test:browser` when local prerequisites are satisfied
5. local sample smoke checks against `samples/local/iwanna-examples/` when relevant assets exist

When changing code, run the narrowest relevant test first, then the broader suite.
```

- [ ] **Step 5: Add explicit documentation-maintenance rules**

Append this section near the change-guideline area:

```md
## Documentation Maintenance Rules

Documentation is part of the implementation, not follow-up cleanup.

When repository reality changes in a meaningful way, update the relevant docs in the same change. This applies to:

- project phase changes
- architecture direction changes
- package-format or runtime-contract changes
- setup or verification command changes
- important crate/layout changes
- changes to current runtime blockers or gold-sample expectations

Required behavior:

- if an older document is no longer current, mark it clearly as `historical` or `superseded`, or remove it if it no longer provides useful context
- treat `README.md`, `AGENTS.md`, and `docs/notes/runtime-wasm-gap-analysis.md` as high-priority always-current docs
- for parser, runtime, or package-contract changes, check whether `README.md` and the relevant `docs/notes/*` files also need updates
```

- [ ] **Step 6: Run a targeted stale-claim check against the updated `AGENTS.md`**

Run:

```powershell
rg "documentation-first|most code has not been created|no bootstrapped Cargo workspace|do not assume those directories already exist" AGENTS.md
```

Expected:

```text
no matches
```

- [ ] **Step 7: Commit the `AGENTS.md` rewrite**

```bash
git add AGENTS.md
git commit -m "docs: refresh agent guide for current repo state"
```

### Task 3: Update Current Top-Level And Active Note Documents

**Files:**
- Modify: `README.md`
- Modify: `samples/README.md`
- Modify: `vendor/README.md`
- Modify: `docs/notes/runtime-wasm-gap-analysis.md`
- Modify: `docs/notes/runtime-gold-sample.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/opengmk-host-coupling-audit.md`
- Modify: `docs/notes/sample-corpus.md`

- [ ] **Step 1: Add a documentation-status note to the README**

Add this short section after the current phase or repository contents area:

```md
## Documentation Notes

Current-state documents should be read as the primary project guide:

- `README.md`
- `AGENTS.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-gold-sample.md`

Older plan documents under `docs/superpowers/plans/` may remain for historical context, but some are now superseded and should say so near the top.
```

- [ ] **Step 2: Update `samples/README.md` to reflect current usage**

Add or replace the purpose section with:

```md
## Current Usage

These samples are used for:

- detector validation
- parser/package smoke testing
- runtime package generation
- runtime regression checks

Important:

- the local sample directories may be absent in a fresh clone
- generated runtime packages under `runtime/public/packages/` are not a substitute for the raw local sample corpus
```

- [ ] **Step 3: Update `vendor/README.md` to reflect current runtime usage**

Add a short note like:

```md
## Current Repository Reality

The project now uses vendored references for both parser and runtime guidance:

- `gm8exe` remains the intended narrow parser dependency boundary
- `gm8emulator` is the primary runtime-semantics reference for the WASM-first runtime path

When runtime-semantics assumptions change, update the relevant runtime notes alongside code changes.
```

- [ ] **Step 4: Add current-status framing to `docs/notes/package-format-v1-runtime.md`**

Insert a short top note:

```md
> **Current status note:** This is the active package-format note.
>
> The older V0 package note is obsolete and should not be used as the current contract.
```

- [ ] **Step 5: Add current-status framing to `docs/notes/runtime-wasm-gap-analysis.md`**

Insert a short top note:

```md
> **Current status note:** Keep this document synchronized with actual runtime-core, runtime-web, and shell behavior.
>
> If code changes reduce or introduce playable-runtime blockers, update this note in the same change.
```

- [ ] **Step 6: Add current-status framing to `docs/notes/runtime-gold-sample.md`**

Insert a short top note:

```md
> **Current status note:** This is an active runtime-priority document, not a historical note.
>
> When the primary gold sample, local package availability, or proven blocker list changes, update this file in the same change.
```

- [ ] **Step 7: Add maintenance notes to `docs/notes/opengmk-host-coupling-audit.md` and `docs/notes/sample-corpus.md`**

Add short notes that:

- `opengmk-host-coupling-audit.md` must stay aligned with actual host-boundary extraction decisions
- `sample-corpus.md` must stay aligned with actual local sample paths and testing expectations

Use content like:

```md
> **Maintenance note:** Update this file when the runtime host boundary or sample workflow changes enough that existing guidance becomes misleading.
```

- [ ] **Step 8: Run a focused doc grep for stale high-level claims**

Run:

```powershell
rg "documentation-first|most code has not been created|no bootstrapped Cargo workspace|future commands|TS-first gameplay-runtime direction|current shell only a static viewer" README.md samples/README.md vendor/README.md docs/notes
```

Expected:

```text
either no matches, or only matches inside intentionally historical text blocks that have already been clearly framed
```

- [ ] **Step 9: Commit the current-doc updates**

```bash
git add README.md samples/README.md vendor/README.md docs/notes
git commit -m "docs: align current notes with repository reality"
```

### Task 4: Mark Historical And Superseded Plan Documents

**Files:**
- Modify: `docs/superpowers/plans/2026-05-19-gm8-detector-foundation.md`
- Modify: `docs/superpowers/plans/2026-05-19-gm8-parser-and-package-builder.md`
- Modify: `docs/superpowers/plans/2026-05-19-runtime-shell-and-static-room-viewer.md`
- Modify: `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md`
- Modify: `docs/superpowers/plans/2026-05-20-wasm-browser-input-render-loop.md`

- [ ] **Step 1: Add a historical banner to the detector-foundation plan**

Insert at the top, immediately under the title:

```md
> **Status note:** Historical implementation plan.
>
> The detector foundation described here has already been implemented in the repository.
> Keep this document for historical context and implementation provenance, not as the primary current task list.
```

- [ ] **Step 2: Add a historical banner to the parser-and-package-builder plan**

Insert at the top:

```md
> **Status note:** Historical implementation plan.
>
> The parser/package-builder foundation described here has already been implemented and extended beyond this document.
> Use current repository contents and current package/runtime notes as the primary source of truth.
```

- [ ] **Step 3: Add a superseded banner to the static-room-viewer plan**

Insert at the top:

```md
> **Status note:** Historical runtime plan.
>
> This document reflects the earlier static-room-viewer stage.
> The repository has already moved beyond this phase into a WASM-first runtime direction.
> Use current runtime notes and newer runtime plans for active work.
```

- [ ] **Step 4: Keep and tighten the superseded banner in the old minimal-playable-runtime plan**

If needed, replace the current top note with:

```md
> **Status note:** Superseded runtime-direction plan.
>
> This document reflects the previous TS-first gameplay-runtime direction.
> It may still contain useful shell, diagnostics, or runtime-slice ideas, but it is no longer the primary implementation route.
> If this document conflicts with `README.md`, current runtime notes, or newer WASM-first runtime plans, follow the current repository state.
```

- [ ] **Step 5: Add a historical or superseded banner to the old wasm-browser-input-render-loop plan**

Insert at the top:

```md
> **Status note:** Historical implementation plan.
>
> This document captures an intermediate runtime step and should be read as project history unless its tasks still match current repository reality.
```

- [ ] **Step 6: Run a grep to confirm the targeted plans are explicitly labeled**

Run:

```powershell
rg "Status note:" docs/superpowers/plans/2026-05-19-gm8-detector-foundation.md docs/superpowers/plans/2026-05-19-gm8-parser-and-package-builder.md docs/superpowers/plans/2026-05-19-runtime-shell-and-static-room-viewer.md docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md docs/superpowers/plans/2026-05-20-wasm-browser-input-render-loop.md
```

Expected:

```text
one explicit status-note match in each targeted file
```

- [ ] **Step 7: Commit the historical-plan labeling**

```bash
git add docs/superpowers/plans
git commit -m "docs: label historical runtime and bootstrap plans"
```

### Task 5: Remove Low-Value Obsolete Package-Format Documentation

**Files:**
- Delete: `docs/notes/package-format-v0.md`
- Modify: `docs/notes/package-format-v1-runtime.md`

- [ ] **Step 1: Confirm that V1 already carries the active package contract**

Run:

```powershell
rg "active package-format note|current contract|runtime-consumable|resources/index.json|scripts.ir.json" docs/notes/package-format-v1-runtime.md
```

Expected:

```text
the V1 note already contains the active contract description or the top status note added earlier
```

- [ ] **Step 2: Delete the obsolete V0 note**

Remove:

```text
docs/notes/package-format-v0.md
```

- [ ] **Step 3: Run a grep to ensure remaining docs no longer point readers to the deleted V0 note as current guidance**

Run:

```powershell
rg "package-format-v0\\.md|scripts\\.json|V0 package" README.md AGENTS.md docs samples vendor
```

Expected:

```text
either no matches, or only intentionally historical references that clearly state V0 is obsolete
```

- [ ] **Step 4: Commit the V0 note removal**

```bash
git add docs/notes/package-format-v1-runtime.md
git rm docs/notes/package-format-v0.md
git commit -m "docs: remove obsolete package format v0 note"
```

### Task 6: Final Verification For Documentation Governance Cleanup

**Files:**
- Modify: none

- [ ] **Step 1: Run a repo-wide stale-claim scan**

Run:

```powershell
rg "documentation-first|most code has not been created|no bootstrapped Cargo workspace|do not assume those directories already exist|future commands|TS-first gameplay-runtime direction" AGENTS.md README.md docs samples vendor
```

Expected:

```text
no misleading current-state matches remain, or any remaining matches are inside clearly marked historical/superseded documents
```

- [ ] **Step 2: Run a repo-wide status-label scan**

Run:

```powershell
rg "Status note:|Maintenance note:" docs AGENTS.md README.md samples vendor
```

Expected:

```text
current docs show maintenance notes where intended, and historical plans show status notes where intended
```

- [ ] **Step 3: Run formatting-neutral verification by checking git diff scope**

Run:

```powershell
git diff --stat HEAD~4..HEAD
```

Expected:

```text
changes are limited to documentation files and intended removals for this cleanup pass
```

- [ ] **Step 4: Commit any final cleanups if needed**

```bash
git add -A
git commit -m "docs: finish documentation governance cleanup"
```

## Self-Review

Spec coverage for this plan:

- current-doc alignment: covered
- `AGENTS.md` rewrite and doc-maintenance rules: covered
- historical/superseded plan labeling: covered
- low-value obsolete doc deletion: covered
- repo-wide verification for misleading stale claims: covered

Placeholder scan:

- no `TODO`
- no `TBD`
- no “update as needed” placeholders without explicit content
- every modified document has a concrete target section or banner text

Type consistency notes:

- `Status note:` is the canonical historical/superseded label prefix
- `Maintenance note:` is the canonical always-current maintenance reminder prefix
- `docs/notes/package-format-v1-runtime.md` remains the active package-format note
- `docs/notes/package-format-v0.md` is removed in this cleanup
