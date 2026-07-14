# Runtime React Shell Design

> **Status: Historical shell design.** The resulting diagnostics surface is
> retained at `/shell`; static fallback described below is intentionally not
> part of the public Beta page.

> **Status:** Historical / implemented
>
> The runtime shell now uses the React structure described here. Keep this file
> as design history; use `runtime/src/ui/`, current tests, `README.md`, and
> `docs/notes/runtime-wasm-gap-analysis.md` for current behavior.

## Overview

This document defines the UI-layer migration for `runtime/` from the current command-style DOM shell to a React-based shell built on the existing Vite app.

The goal of this change is not to alter runtime semantics or shell feature scope. The goal is to:

- keep the current runtime shell feature set intact
- reduce UI code volume and structural complexity
- make the shell easier for future agent-driven maintenance
- replace the current debug-panel-heavy presentation with a copy-first plain-text diagnostic report

This is a shell and developer-experience change. It does not change the normalized runtime package contract, the WASM bridge contract, or the runtime-core execution path.

## Goals

- Keep current shell functionality available:
  - package path input
  - package loading
  - room selection
  - pause and resume
  - reset
  - canvas rendering for static viewer and WASM path
  - HUD visibility for current runtime state
  - package inspector visibility
- Rebuild UI composition with React instead of imperative DOM assembly
- Move shell styling to Tailwind CSS as the default UI styling path
- Use `shadcn/ui` only where it meaningfully reduces repetitive UI scaffolding
- Replace the current multi-panel debug presentation with a primary plain-text report that is easy to read and easy to copy in one action
- Preserve current runtime integration modules unless there is a direct migration need

## Non-Goals

- Do not change runtime-core semantics
- Do not redesign the package format
- Do not replace canvas rendering with React rendering
- Do not remove existing shell capabilities to simplify the migration
- Do not expand shell functionality beyond current feature scope in the same implementation pass
- Do not introduce a broad new component-library dependency surface unless it directly reduces local complexity

## Current State

The current `runtime/` app is already a Vite + TypeScript application. It is not a non-Vite shell.

The current shell architecture uses:

- `src/main.ts` as the Vite entrypoint
- imperative DOM construction in `src/ui/shell.ts`
- imperative HUD/debug/inspector rendering helpers in:
  - `src/ui/hud.ts`
  - `src/ui/debugPanels.ts`
  - `src/ui/inspectors.ts`
  - `src/ui/traceView.ts`
- existing runtime integration and renderer modules in:
  - `src/loadPackage.ts`
  - `src/runtime/wasmBridge.ts`
  - `src/runtime/wasmSession.ts`
  - `src/render/staticRoomRenderer.ts`
  - `src/render/wasmFrameRenderer.ts`

The migration should be understood as:

- keep Vite
- replace imperative shell UI assembly with React
- simplify debug presentation
- keep runtime behavior modules stable where possible

## Design Summary

The shell will move to a React-controlled page structure while preserving the existing runtime integration boundaries.

The new shell will:

- mount a React app from `src/main.ts`
- keep the canvas renderer outside React's drawing model
- centralize shell state in React hooks
- derive HUD and diagnostics display from explicit state and formatting helpers
- present a single primary copy-first debug report instead of relying on multiple collapsed detail panels for the main debugging workflow

The recommended architecture is:

1. keep runtime-facing modules and renderers mostly unchanged
2. replace `src/ui/*` imperative DOM builders with React components and hooks
3. convert shell styling from hand-authored page CSS to Tailwind-first layout and utility classes
4. keep package inspectors as read-only shell tooling, but present them in a lighter React structure such as tabs or segmented views

## Architecture

### Kept As-Is Unless Directly Required

These modules should remain non-React runtime modules:

- `src/loadPackage.ts`
- `src/runtime/wasmBridge.ts`
- `src/runtime/wasmSession.ts`
- `src/render/staticRoomRenderer.ts`
- `src/render/wasmFrameRenderer.ts`
- `src/runtime/wasmAudioHost.ts`

Reasoning:

- they already define the runtime/package boundary
- they are not the source of current shell maintenance complexity
- converting them to React-aware modules would add coupling without improving the runtime architecture

### Replaced Or Reshaped

The imperative UI helpers will be replaced with React-based presentation and derived-state helpers:

- `src/main.ts` becomes a React mount entry
- `src/ui/shell.ts` is replaced by a React page-level container or decomposed into `src/ui/components/*` plus `src/ui/hooks/*`
- `src/ui/hud.ts` becomes a React HUD component
- `src/ui/debugPanels.ts` is superseded by a copy-first debug report component and supporting formatters
- `src/ui/inspectors.ts` becomes React inspector views
- `src/ui/traceView.ts` is reduced to formatting and derived-state responsibilities where still useful

### React Component Structure

Recommended structure:

- `App`
  - top-level shell mount
- `RuntimeShellPage`
  - page layout and overall state wiring
- `ControlBar`
  - package path, load action, room selection, pause/resume, reset
- `CanvasStage`
  - owns canvas ref and rendering boundary
- `RuntimeHud`
  - current eight summary cards, rendered from props
- `DebugReportPanel`
  - primary text report surface with one-click copy
- `InspectorTabs`
  - read-only package/room/object/script summary views

Supporting hooks:

- `useRuntimeShell`
- `useKeyboardInput`
- `useDebugReport`
- `useInspectorData`

Supporting helpers:

- formatters for status lines, performance summaries, tick-phase text, runtime event text, and inspector summaries

## State Model

### Runtime State

The main hook should own shell runtime state, including:

- loaded package
- active backend mode:
  - static viewer
  - WASM runtime
- last snapshot
- last frame/performance stats
- current room selection
- auto-tick running state
- loading state
- recoverable error state

### UI State

The UI layer should own view-specific state, including:

- inspector tab selection
- debug text wrap toggle
- copy feedback state
- optional panel open/closed state where needed

### Derived State

Derived state should be calculated, not manually duplicated:

- HUD card text
- current status label
- room label
- player summary
- diagnostics summary
- recent runtime events
- performance summary
- tick-phase summary
- copyable plain-text debug report

## Data Flow

The data flow should remain simple and explicit:

1. user loads a package
2. shell loads normalized package data
3. shell attempts WASM boot
4. if WASM boot succeeds:
   - shell creates a `WasmRuntimeSession`
   - shell drives tick and draw loops
   - shell stores latest snapshot and performance state
5. if WASM boot fails:
   - shell falls back to static room viewer
   - shell stores the failure reason for visible diagnostics
6. React components render from current state
7. `CanvasStage` uses state changes to trigger the existing renderer against the canvas element
8. `useDebugReport` converts runtime state into stable copyable text

Important boundary:

- React manages state and orchestration
- existing renderer modules still draw into the real canvas

## Debug Report Design

### Primary UX Change

The current debug display is useful for browsing but is not optimized for quick sharing or one-shot copying.

The new primary debugging surface should be a single readable plain-text report that:

- is visible without opening multiple nested panels
- is readable in-place
- can be copied in one action
- is stable enough to paste into chat, issues, or review notes

### Report Scope

The report should cover:

- runtime status
- room identity
- tick
- player state
- input state
- performance summary
- tick phases
- recent runtime events
- diagnostics list

It should not include full package inspector dumps by default.

### Report Format

The report should prefer stable text sections over JSON.

Representative shape:

```text
Status: WASM runtime active
Room: 143 sampleroom01
Tick: 240

Player:
- x=123.5 y=456
- hspeed=3.2 vspeed=-7.8
- object=player#17
- alive=true grounded=false
- jumpActive=true hold=4 cut=false

Input:
- jumpKey=0x10
- pressed=false justPressed=false justReleased=true
- keys=[16,39]

Performance:
- total=14.8ms budget=ok skipped=0 commands=128
- input=0.1 tick=8.3 snapshot=1.4 frame=2.2 render=2.8 runtime=12.0

Tick Phases:
- total=8.113ms
- inputDiag=0.011 step=4.201 view=0.042 player=0.388
- collision=1.102 alarms=0.030 keyboard=0.205 renderSubmit=2.134

Recent Events:
- runtime-instance-created ...
- runtime-room-changed ...

Diagnostics:
- ...
- ...
```

Format rules:

- fixed section order
- stable field labels
- no nested JSON by default
- short line lengths where practical
- deterministic wording so diffs stay readable

### Copy Behavior

`DebugReportPanel` should provide:

- a visible `Copy` action
- direct clipboard copy of the exact rendered report text
- lightweight success feedback

Optional lightweight controls are acceptable, such as:

- wrap on/off
- monospace display toggle

The design should avoid rebuilding the previous debug UX as many separate collapsible panels unless a specific panel remains necessary for feature parity.

## HUD Design

The current shell exposes eight summary areas. That summary surface should remain available after migration.

Recommended card set:

- status
- room
- tick
- player
- input
- events
- diagnostics
- frame

The React HUD should:

- render from derived props only
- avoid embedding shell orchestration logic
- preserve the shell's current manual-testing emphasis

## Inspector Design

Package inspectors remain part of the shell feature set and should stay available.

However, the presentation can be simplified:

- move away from nested `details`-based presentation as the primary structure
- use tabs or segmented controls for:
  - package
  - rooms
  - objects
  - scripts

Inspector content should remain summarized and sliced rather than dumping full package files into the page by default.

## Styling Strategy

### Tailwind First

Tailwind CSS becomes the default styling path for the React shell.

Expected usage:

- layout primitives
- spacing
- typography
- color
- borders
- responsive structure
- interactive states

### Minimal CSS Residue

The current `styles.css` should be reduced to:

- Tailwind entry imports
- minimal base-layer setup
- any narrow non-utility rules that are still justified

The migration goal is to reduce page-level custom CSS ownership, not to re-encode the current CSS complexity in another file.

### shadcn/ui Policy

`shadcn/ui` is allowed where it clearly reduces boilerplate for simple interface pieces such as:

- buttons
- tabs
- textarea-like text surface wrappers

It should not be introduced as a large new abstraction layer just because the shell is moving to React.

Decision rule:

- use `shadcn/ui` only when it shortens local code and stays easy to own
- avoid broad dependency spread for low-value visual polish

## Error Handling

The shell should explicitly model at least these top-level states:

- `idle`
- `loading`
- `ready`
- `load_failed`
- `runtime_failed`

### Load Failure

If package loading fails:

- keep controls usable for retry
- show failure reason clearly
- render a recoverable error state

### WASM Boot Failure

If WASM bridge boot fails:

- fall back to static room viewer
- keep failure reason visible in shell status
- include the failure reason in the copyable debug report

### Runtime Tick Failure

If auto-tick fails after boot:

- stop auto-tick
- preserve last available shell state
- append visible failure diagnostics
- keep the shell recoverable through reload/reset where practical

## Testing Strategy

The migration should preserve the current test stack:

- Vitest for unit-level and component-level tests
- Playwright for browser-shell smoke coverage

### Tests To Keep

Keep runtime-facing tests for:

- `loadPackage.*`
- `wasmBridge.*`
- `wasmSession.*`
- renderer modules
- audio host modules

### Tests To Replace Or Add

The old DOM-helper-oriented UI tests should move toward React component and hook tests:

- `useDebugReport` formatting tests
- `RuntimeHud` rendering tests
- `ControlBar` interaction tests
- shell page rendering tests for:
  - viewer mode
  - wasm mode
  - error mode

### Browser Smoke

Browser smoke should continue covering:

- package load
- WASM boot status visibility
- room/tick/player visibility
- debug report visibility
- one-shot copy path for the debug report

## Documentation Updates

This migration changes shell implementation reality and shell debugging UX, so documentation must update in the same change.

Required updates:

- `README.md`
  - reflect React + Tailwind-based `runtime/` shell implementation
  - update shell debugging UX wording if it still describes collapsed debug panels as the primary path
- `docs/notes/runtime-wasm-gap-analysis.md`
  - update shell telemetry/debugging descriptions to match the new copy-first report

Update other notes only if their current statements become inaccurate.

## Implementation Constraints

- Keep the feature surface stable in this pass
- Keep runtime semantics out of the React migration scope
- Keep renderer ownership on the canvas path, not in React markup
- Keep formatter output stable enough for repeated copy/paste comparisons
- Prefer small focused files over one large replacement `shell.tsx`

## Recommended File Direction

One reasonable target structure is:

- `src/main.tsx`
- `src/app/App.tsx`
- `src/ui/components/ControlBar.tsx`
- `src/ui/components/CanvasStage.tsx`
- `src/ui/components/RuntimeHud.tsx`
- `src/ui/components/DebugReportPanel.tsx`
- `src/ui/components/InspectorTabs.tsx`
- `src/ui/hooks/useRuntimeShell.ts`
- `src/ui/hooks/useKeyboardInput.ts`
- `src/ui/hooks/useDebugReport.ts`
- `src/ui/hooks/useInspectorData.ts`
- `src/ui/formatters/*`

The exact file split can vary, but the important architectural rule is:

- components render
- hooks orchestrate
- formatters derive text
- runtime modules execute

## Tradeoffs

### Why Not Keep The Imperative DOM Shell

Keeping the current shell and only polishing the debug panels would be lower risk, but it would preserve the main maintainability problem:

- UI state, orchestration, and DOM assembly are too interleaved

That is the part most likely to remain expensive for future agent-driven maintenance.

### Why Not Collapse The Shell Into A Single Console View

A console-first shell would make copying easier, but it would shift the shell away from the current feature surface and debugging workflows.

That would violate the current requirement to keep functionality intact during this pass.

### Why React + Tailwind

This combination is appropriate here because:

- React gives cleaner UI state composition
- Tailwind reduces page-specific CSS maintenance
- the shell is an operational interface, not a marketing page
- the canvas-based runtime display already fits a React-orchestrated, non-React-rendered drawing model

## Rollout Plan

Implementation should proceed in this order:

1. add React and Tailwind infrastructure to `runtime/`
2. replace entrypoint mount with React
3. build the shell state hook around existing runtime modules
4. migrate control bar and canvas stage
5. migrate HUD
6. implement copy-first debug report and formatter tests
7. migrate inspectors
8. remove old imperative UI helpers or reduce them to pure helpers where still justified
9. update docs
10. run targeted tests first, then runtime browser smoke

## Acceptance Criteria

This design is satisfied when:

- `runtime/` uses React as its UI composition layer
- shell styling is primarily Tailwind-based
- current shell functionality remains available
- the canvas still renders both viewer and WASM runtime output correctly
- debug information is available as one readable plain-text report with one-shot copy
- browser and unit tests cover the new UI structure
- docs reflect the new shell implementation and debugging workflow
