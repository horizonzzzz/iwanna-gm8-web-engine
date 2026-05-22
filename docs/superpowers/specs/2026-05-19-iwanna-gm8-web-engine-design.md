# IWanna GM8 Web Engine MVP Design

## Overview

This document defines the first-phase design for a browser-playable IWanna engine targeting the mainstream legacy Game Maker 8.x fangame ecosystem.

The project goal is not to emulate all Game Maker games, and not to support every fangame engine from the start. The first goal is narrower:

- Accept original uploaded game packages from users
- Identify whether a package is likely a supported GM8-style IWanna game
- Convert supported games into a normalized internal package
- Run that package in a browser through a WASM-first runtime path
- Optimize for "can start and play core gameplay" before "perfect behavioral parity"

The project will use a backend-assisted pipeline. The browser will not directly parse every possible GM8 executable format on its own.

## Product Goal

Build an MVP that demonstrates a credible end-to-end path from original fangame distribution files to a browser-playable experience.

The MVP is intended to answer these questions:

1. Can the system reliably detect mainstream GM8 IWanna games?
2. Can the system extract enough structure from those games to produce a normalized runtime package?
3. Can a browser-hosted runtime path execute enough real runner behavior to support a meaningful portion of classic fangames?
4. Can the system classify unsupported games cleanly instead of failing opaquely?

The MVP is not required to:

- Support all Game Maker versions
- Support non-GM engines such as RPG Maker, Clickteam, Unity, GMS2, Godot, NW.js, or custom engines
- Guarantee full trap timing parity or pixel-perfect reproduction in phase one
- Guarantee that every uploaded game is finishable
- Guarantee support for complex external DLL behavior

## Scope

### In Scope

- Legacy GM8-style mainstream IWanna fangames
- Single-exe and multi-file GM8 distribution packages
- Backend parsing, normalization, and compatibility analysis
- Browser-hosted runtime execution path for a compatibility subset, with room to move toward deeper runner fidelity
- Upload flow for user-provided original game packages
- Sample corpus management and classification
- Compatibility reporting and runtime diagnostics

### Out of Scope

- Native execution of uploaded EXEs on the backend as a cloud-streamed game service
- Complete Game Maker emulation
- Support for obviously non-target engines
- Multiplayer
- User accounts, social features, and permanent product decisions
- Anti-cheat or competitive validation

## High-Level Strategy

Three broad implementation strategies were considered:

1. Full browser-side GM8 runtime recreation
2. Backend parsing plus normalized package outputs plus a WASM-first browser runtime
3. Server-side native execution with browser streaming

The selected strategy is:

### Backend parsing plus normalized package outputs plus a WASM-first browser runtime

Users upload original game packages. The backend identifies and parses target games, converts them into a normalized internal package format, and ships that format to a browser-hosted runtime path. The browser-facing `runtime/` app acts as the shell, diagnostics surface, and host glue, while the long-term execution engine is a WASM-hosted runtime core rather than a project-owned TypeScript gameplay reimplementation.

This is selected because it balances:

- user experience: users still upload original files
- technical control: heavy parsing and compatibility analysis stay on the backend
- long-term extensibility: the normalized package and shell can remain stable while parser and runtime fidelity improve
- MVP feasibility: avoids requiring the browser to parse raw executables directly while preserving a path toward deeper runner-level behavior

## Supported Input Model

The system should not model input as "single EXE only".

It should support the idea of a game package containing:

- one primary EXE
- optional DLLs
- optional audio files
- optional image files
- optional INI or TXT files
- optional other auxiliary files

The upload unit should therefore be either:

- a single EXE file, or
- a compressed archive containing a game directory

Internally, the backend should reason about "a package" rather than "a file".

## Engine Target Definition

The target category for phase one is:

- mainstream legacy IWanna fangames that are likely built on GM8.x runners or equivalent classic GM-style packaging

This means the real problem is not "support all fangames", but:

- identify GM8-compatible candidates
- reject obvious non-target engines
- degrade gracefully on edge cases

### Important Distinction

Do not equate these two ideas:

- multi-file distribution
- non-GM engine

Many GM8 games still ship with:

- `dll`
- `ogg`
- `wav`
- `mp3`
- `ini`
- `txt`

Those remain viable targets as long as the game logic is still fundamentally GM8-compatible.

## Design Principles

### 1. Prioritize startup and core playability first

Compatibility progress should first optimize for:

- can identify package
- can parse package
- can enter game
- can control player
- can die and respawn
- can change rooms

This is more valuable early than pursuing perfect parity on one extremely complex game.

### 2. Keep backend and runtime responsibilities separate

The backend should solve parsing and normalization problems.

The browser runtime path should solve execution problems against a stable internal package format, but the outer browser shell and the execution engine should be treated as separate responsibilities.

### 3. Treat compatibility as a measured process

The system should report:

- what was identified
- what APIs were found
- what features are unsupported
- what failed during execution

Compatibility growth should be data-driven from a sample corpus rather than anecdotal.

### 4. Preserve future extension paths

The MVP should be explicitly designed so it can later expand to:

- more GM8 features
- more external resource forms
- more robust runtime data and execution inputs where required by the WASM runtime path
- possibly more engine families

without redesigning the entire project structure.

## System Architecture

The system is divided into four layers.

### 1. Upload and Analysis Layer

Responsibilities:

- receive uploaded EXE or archive
- unpack archive if needed
- identify main executable
- compute hashes
- perform basic safety checks
- identify likely engine family
- produce a first-pass compatibility verdict

Outputs:

- package metadata
- engine verdict
- file inventory
- detected external resources
- detected DLL dependencies

### 2. Normalization and Compilation Layer

Responsibilities:

- parse GM8 game data from the executable
- extract sprites, sounds, rooms, objects, scripts, and event definitions
- collect external files that are actually needed
- translate logic into a normalized internal representation
- generate package manifest and compatibility analysis

Outputs:

- normalized internal package
- analysis report
- unsupported feature report

### 3. Browser Runtime Layer

Responsibilities:

- load normalized package
- manage fixed-step update loop
- instantiate rooms and objects
- evaluate supported logic IR
- process input, collisions, audio, rendering, and room changes
- handle error reporting and runtime diagnostics

### 4. Compatibility and Observability Layer

Responsibilities:

- record parser warnings
- record unsupported APIs
- record runtime errors
- record whether rooms load successfully
- record whether player control, death, respawn, and room transitions work

This layer is critical. Without explicit observability, the project will devolve into patching one-off game failures without understanding compatibility trends.

## Backend Responsibilities

The backend should own everything related to original package interpretation.

### Backend responsibilities include:

- upload handling
- archive expansion
- EXE discovery
- sample classification
- engine detection
- GM8 parsing
- extraction of game data
- external resource inventory
- external DLL inventory
- compatibility analysis
- logic normalization
- package building
- caching normalized package outputs by source hash

### Backend should not depend on:

- native execution of uploaded games
- screen streaming
- sandboxed real-time gameplay as the primary approach

## Browser Runtime Responsibilities

The browser runtime path should only need to understand the normalized internal package.

### Runtime responsibilities include:

- package loading and validation in the browser shell
- fixed-timestep host control around the runtime core
- keyboard input capture and host injection
- room loading and runtime boot
- instance lifecycle
- runtime-driven drawing through a browser-consumable frame surface
- collision handling
- audio playback or explicit no-op diagnostics when unsupported
- death and respawn
- minimal runtime debug overlays and diagnostics

The runtime should not be tightly coupled to raw GM8 executable structure.
The frontend shell remains useful as an inspection tool, but gameplay execution now belongs to the WASM runtime path, with static room viewing as the non-WASM fallback.

## Internal Package Format

The backend should output a custom package format referred to here as `iwm`.

This name is only provisional, but a project-owned format is important.

The package should logically contain the following parts:

- `manifest.json`
- `resources/`
- `resources/index.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `logic.raw.json`
- `logic.lowered.json`
- `analysis.json`

The package may eventually be compressed as a single archive, but its logical structure should remain explicit.

The current repository direction keeps both script inventory and parser-owned logic artifacts in the runtime-facing package:

- `scripts.ir.json` remains the structural script export
- `logic.raw.json` preserves original GML for provenance and diagnostics
- `logic.lowered.json` carries the structured lowered contract used by the current runtime-facing path

### Phase 2 V0 package output

Before resource export and IR lowering are implemented, the first package-builder milestone should emit a structural summary package directory containing:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.json`
- `analysis.json`

This V0 output is intentionally not runtime-ready.

Its purpose is to:

- validate parser integration against real GM8 samples
- stabilize JSON output shape for downstream tooling
- expose compatibility and unsupported-feature data early

In V0, `scripts.json` is a structural summary file rather than executable IR.

Once logic lowering and browser-friendly resource export exist, V0 can evolve or be superseded by the runtime-facing package layout described below.

### manifest.json

Purpose:

- package-level metadata
- source provenance
- versioning
- compatibility flags

Suggested fields:

- package format version
- original game name
- original file hash
- detected engine verdict
- parser version
- build timestamp
- compatibility level: `supported`, `partial`, `blocked`
- required runtime capabilities
- package warnings summary

### resources/

Purpose:

- browser-friendly resource storage

Suggested contents:

- sprite frames or atlases
- background images
- audio assets
- optional derived metadata for frame bounds and origins

Design note:

The package should optimize for browser loading and runtime simplicity rather than preserve original GM8 storage layout.

### rooms.json

Purpose:

- normalized room definitions

Suggested contents per room:

- room id
- room name
- width and height
- speed
- background configuration
- view or camera metadata
- initial instance placements
- room creation logic reference

### objects.json

Purpose:

- normalized object definitions

Suggested contents per object:

- object id
- object name
- parent object reference
- sprite reference
- depth
- visibility
- persistence
- event table

Each event entry should reference an IR block rather than relying on raw GML text in the browser.

### scripts.ir.json

Purpose:

- normalized executable logic

This file is part of the runtime-facing package target, not the initial V0 structural package.

This is the most important interface in the entire architecture.

The browser should not need to parse arbitrary GM8 source text if that can be avoided.

Instead, the backend should lower supported logic into an internal IR.

Suggested initial IR capabilities:

- variable read
- variable write
- arithmetic
- comparisons
- branching
- function calls
- instance create and destroy
- room transitions
- keyboard queries
- collision queries
- sound play and stop

The IR can remain intentionally narrow in the MVP as long as unsupported constructs are explicitly reported.

### analysis.json

Purpose:

- parser and compatibility report

Suggested contents:

- detected APIs
- unsupported APIs
- detected external DLL calls
- asset extraction warnings
- classification notes
- runtime support warnings

This file should help answer why a game works, partially works, or fails.

## Logic Execution Model

The browser runtime path should start from a constrained compatibility target, but the strategic execution direction is no longer a handwritten TypeScript subset model. Runtime fidelity should accumulate in the WASM-hosted engine path.

### Phase-one target

A deliberately constrained early compatibility target focused on mainstream fangame patterns:

- player movement logic
- object trigger logic
- collisions
- traps
- bullets
- saves and respawns
- room changes
- basic menu or title transitions if feasible

### Not required in phase one

- complete GML syntax support
- all legacy edge-case behaviors
- advanced particle systems
- broad arbitrary DLL semantics
- every obscure event type

## Compatibility Model

Uploaded packages should resolve to one of several explicit states.

### supported

The package appears targetable and the current normalized package plus browser-hosted runtime path is expected to run core gameplay, subject to current runtime fidelity limits.

### partial

The package is targetable, but uses unsupported APIs or features that may block complete playability.

### blocked

The package is clearly outside the supported engine family or requires capabilities the system intentionally does not provide.

### unknown

The package does not provide a strong enough signal for confident classification and needs manual review or improved heuristics.

## Sample Corpus Strategy

Sample management is a core development asset, not an afterthought.

The project should maintain a corpus of representative examples grouped by compatibility risk.

The current local sample set has already been organized into:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

### Current intended meaning

#### gm8-core

Likely mainstream GM8 candidates with simpler packaging and lower expected compatibility risk.

#### gm8-extended

Likely GM8 candidates with extra DLL or resource complexity.

#### needs-manual-check

Samples whose automated signals are inconclusive and need deeper inspection.

#### non-target

Games likely outside the MVP engine target, such as Studio ports or modern engine variants.

### Why this matters

The sample corpus should support:

- regression testing
- parser improvement
- runtime compatibility measurement
- engine detection heuristic refinement

## Current Sample Classification

Based on current local inspection, the following samples were classified as `gm8-core`:

- `I wanna afflict the mashikaku`
- `I wanna be the 3200min`
- `I wanna be the Crimson ver.1.0`
- `I wanna be the Experience`
- `i wanna be the Favorite difficulty ver1.1`
- `I wanna be the Forever`
- `I Wanna Kill the Kamilia Ver. Final`
- `IWBT_Dife`

Classified as `gm8-extended`:

- `I wanna be the Agent`
- `I Wanna Break The Fourth Wall`

Classified as `needs-manual-check`:

- `Soft & Wet - Go Beyond`
- `I Wanna Kill The BLB 2 Nemesis Tamatou V1.975b`
- `I Wanna Kill the Kamilia 3 v2.02`
- `Not Another Needle Game 1.12`

Classified as `non-target`:

- `Amaranth Needle Studio Port`
- `I Wanna Be The GBC`

These classifications should be treated as working development labels, not final truth.

## Detection Heuristics

The detection pipeline should use heuristics to classify likely engine family.

### Positive GM8 signals

- `Game Maker`
- `Version 8`
- `D3DX8.dll`
- `room_goto`
- `keyboard_check`

### Likely non-target signals

- `data.win`
- `YoYo Games`
- `UnityPlayer.dll`
- `RPG_RT.exe`
- `Game.rgss`
- `Clickteam`
- `Godot Engine`
- `nw.exe`

### Important note

Heuristics are only a front-end filter. They should not be considered authoritative if a stronger parser-based verdict can be produced later.

## External DLL Policy

The MVP should not promise broad support for arbitrary DLL behavior.

Instead:

- detect and list DLLs
- determine whether gameplay appears to depend on them
- support packages whose core gameplay still works without deep DLL emulation
- explicitly mark risky dependencies

This allows the project to support many GM8 multi-file packages without overcommitting to impossible general DLL compatibility.

## Error Handling Strategy

Errors should be explicit and categorized.

### Backend errors

- archive invalid
- no executable found
- multiple executable candidates without clear primary
- unsupported engine signature
- parser extraction failure
- unsupported encrypted or malformed game data

### Runtime errors

- unsupported IR opcode
- missing resource reference
- unsupported event type
- room load failure
- fatal execution error in script block

### User-facing behavior

Whenever possible, the user should receive:

- a verdict
- a short reason
- a detailed compatibility report

Silent failure is unacceptable.

## Observability and Metrics

The project should measure at least:

- upload count by verdict
- parse success rate
- package build success rate
- game boot success rate
- room-entry success rate
- player-control success rate
- death and respawn success rate
- missing API frequency
- runtime error frequency by category

These metrics can initially be stored locally in logs or simple structured files. The important part is to collect them consistently.

## MVP Milestones

### Milestone 1: Detector

Goal:

- accept uploaded game file or archive
- classify engine family
- inventory package files
- emit initial compatibility verdict

Success criteria:

- correctly separate obvious `gm8-likely`, `gms-likely`, `unknown`, and `blocked` cases from the sample set

### Milestone 2: Static Package Builder

Goal:

- parse a gold-sample game
- extract room, object, sprite, and event structure
- build a normalized package without executing logic

Success criteria:

- package builder emits deterministic structural summaries and analysis for a gold-sample game
- downstream tooling can inspect manifest, room, object, script, and analysis outputs without re-parsing the original executable
- browser-side static room rendering is deferred until resource export and room-instance normalization exist

### Milestone 3: Minimal Playable Runtime

Goal:

- run one gold-sample game in browser with core interaction

Minimum capability target:

- enter game
- move left and right
- jump
- die
- respawn
- perform basic room transition

### Milestone 4: Core Corpus Compatibility

Goal:

- run the `gm8-core` corpus in batch evaluation

Success criteria:

- report boot rate
- report control rate
- report room-transition rate
- rank missing APIs and unsupported features by frequency

## Gold Sample Recommendation

A single simpler sample should be chosen as the first gold target.

Recommended characteristics:

- likely GM8
- minimal packaging complexity
- straightforward platforming
- room transitions present
- death and respawn easy to validate

The current primary gold sample is `IWBT_Dife` under `samples/local/iwanna-examples/gm8-core/IWBT_Dife`.
If that target changes, `docs/notes/runtime-gold-sample.md` should be updated in the same change.

## Repository and Workspace Expectations

The development project directory for follow-up work is:

- `C:\Users\59164\work\playground\iwanna-gm8-web-engine`

The detailed design spec lives under:

- `docs/superpowers/specs/`

The current project-local sample root is:

- `samples/local/iwanna-examples/`

Vendored parser reference repositories under `vendor/` are tracked as git submodules and should be initialized after clone.

This document is intended to provide enough context for future sessions started in that project directory.

## Current Repository Layout

The current repository already contains:

- `docs/`
- `docs/superpowers/specs/`
- `docs/notes/`
- `samples/`
- `vendor/`
- `crates/iwm-detector/`
- `crates/iwm-parser/`
- `crates/iwm-cli/`
- `crates/iwm-runtime-model/`
- `crates/iwm-runtime-host/`
- `crates/iwm-runtime-core/`
- `crates/iwm-runtime-web/`
- `runtime/`

Later phases may still add:

- `backend/`

## Risks

### 1. GM8 semantic complexity

Even a narrow fangame subset may rely on runner behavior that is more idiosyncratic than expected.

### 2. Parser availability gap

Existing open-source projects may expose useful code or formats, but integrating them into this exact pipeline may still require substantial engineering.

### 3. DLL dependency surprises

Games may appear simple but rely on external DLL behavior for audio, memory, or control flow.

### 4. False positives in engine detection

String heuristics alone are not enough. The parser must eventually become the main source of truth.

### 5. Compatibility scope creep

Without a firm definition of "core gameplay first", the project can collapse into endless edge-case chasing before producing a useful MVP.

## Open Decisions

These items do not block the design, but they still need explicit resolution during future execution work:

- backend upload/orchestration shape and deployment model
- exact host-boundary extraction strategy for OpenGMK-derived runtime code
- exact long-term execution contract beyond the current `logic.lowered.json` subset
- exact browser-host mapping for GM8 input, audio, and render semantics
- archive upload format support details
- licensing and distribution constraints for any OpenGMK-derived runtime artifact

## Recommendation Summary

The recommended path is:

1. build a backend detector and classifier
2. build a backend GM8 normalization pipeline
3. define a stable internal package and logic IR
4. build a WASM-first browser runtime path that can start narrow and then grow toward deeper runner fidelity
5. grow compatibility by measuring the sample corpus, not by intuition

This path preserves the best balance of feasibility, user experience, and long-term technical leverage.
