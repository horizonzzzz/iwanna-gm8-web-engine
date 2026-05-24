# IWBT_Dife Jump Alignment Design

## Overview

This document defines the first focused jump-alignment design for the WASM-first runtime path.

The goal is not to build a generic platformer jump model or to guess at "IWanna-like" feel from scratch.
The goal is to align the runtime's jump behavior with the practical gold-sample target:

- `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

This design treats jump fidelity as a runtime-core semantics problem with sample-driven validation.
OpenGMK remains the upstream semantic reference for GM8 runner behavior, but it is not the sole authority on final jump feel because the effective jump arc in IWanna games also depends on game script variables and event timing.

## Product Goal

Build a jump-alignment slice that moves the runtime from a fixed-height placeholder jump toward a jump model that is credible against `IWBT_Dife`.

For this slice, "credible" means all of the following are true:

- shorter jump-key holds produce a lower jump than longer holds
- early jump release cuts upward motion in a way that matches the sample's behavior directionally
- head bumps against ceilings trim or cancel upward motion in the correct phase
- landing resets jump state cleanly so repeat jumps behave consistently
- jump behavior stays coupled to existing wall and floor collision behavior instead of bypassing it
- validation can compare runtime output against sample-derived frame traces instead of relying on subjective browser feel alone

## Scope

### In Scope

- runtime-core jump-state modeling in `crates/iwm-runtime-core/`
- held / pressed / released jump-input semantics already exposed through the host bridge
- jump-height variation based on jump-key hold duration
- jump-cut behavior on early key release
- ceiling-hit handling during upward travel
- landing reset behavior after airborne state resolves
- jump behavior interacting with current solid collision resolution
- deterministic trace capture for jump-focused runtime validation
- `IWBT_Dife` as the primary acceptance target
- OpenGMK as the GM8 semantics reference for movement/input/event ordering

### Out of Scope

- broad rework of all movement semantics
- full OpenGMK extraction for this slice
- complete GM8 physics parity beyond the jump-critical path
- browser-shell-side gameplay heuristics
- audio, animation, or camera work
- generalized corpus-wide jump tuning before `IWBT_Dife` is aligned
- direct code copying from `vendor/OpenGMK/`

## Why This Slice Exists

The current runtime still uses a placeholder jump path:

- jump starts only from `jump.just_pressed`
- jump height is effectively fixed
- upward motion does not model held-vs-released divergence
- ceiling and landing behavior are only incidentally coupled through the current axis movement helper

That is enough for a smoke slice, but not enough for IWanna gameplay fidelity.
Jumping is one of the highest-value behaviors in the project because a wrong jump arc makes every room feel wrong even when collision, room loading, and input routing are nominally working.

## Reference Model

This design uses two references with different responsibilities.

### 1. OpenGMK as runtime-semantics reference

OpenGMK should guide:

- held / pressed / released input semantics from `vendor/OpenGMK/gm8emulator/src/input.rs`
- keyboard event timing from `vendor/OpenGMK/gm8emulator/src/game/events.rs`
- movement update ordering from `vendor/OpenGMK/gm8emulator/src/game/movement.rs`
- collision-adjacent movement expectations where runner state and object motion interact

OpenGMK is used here to stop the runtime from inventing ad hoc update rules.
It is the baseline for how a GM8-style runtime should treat button edges, gravity application, and motion ordering.

### 2. `IWBT_Dife` as hand-feel acceptance reference

`IWBT_Dife` should decide:

- whether the resulting short-hop and full-hop trajectories are believable
- whether ceiling interaction and landing reset occur at the right frame windows
- whether the runtime's jump variable interpretation is close enough to the sample to continue building on

OpenGMK alone cannot answer this because the final jump feel depends on the actual game's script variables and object logic, not only runner internals.

## Selected Strategy

Three implementation directions were considered:

1. Tune hardcoded jump constants until the browser feel seems close
2. Introduce a jump state machine in runtime-core, use OpenGMK for semantic ordering, and validate against `IWBT_Dife`
3. Delay jump work until a larger OpenGMK-derived runtime extraction is complete

The selected strategy is:

### Runtime-core jump state machine with OpenGMK-guided semantics and `IWBT_Dife`-driven validation

This is selected because it keeps the scope narrow enough to ship now while still aligning with the repository's current runtime direction.

It avoids two bad outcomes:

- browser-shell heuristics that bypass the runtime core
- waiting for a much larger OpenGMK extraction before improving the most player-visible mechanic

## Design Principles

### 1. Keep jump logic in `runtime-core`

The browser shell and web host should only translate input into normalized button states.
They should not implement jump hold windows, short-hop rules, or ceiling-cut behavior.

### 2. Prefer variable-driven runtime behavior over fixed constants

The jump path should prefer player variables already commonly used by IWanna logic, such as:

- `jump`
- `jump2`
- `djump`
- `gravity`
- `maxFallSpeed`

If those values are absent, the runtime may use conservative defaults, but the intended direction is variable-driven behavior rather than a permanently hardcoded jump profile.

### 3. Validate by frame traces, not only by subjective feel

Jump fidelity needs deterministic evidence.
The runtime should emit jump-focused traces that can be compared across:

- tap jump
- held jump
- early-release jump
- ceiling-contact jump
- repeated land-and-jump sequences

### 4. Stay within the current runtime-core architecture

This slice should extend the existing runtime core, movement helpers, and tests.
It should not turn into a broad physics rewrite or a parallel movement subsystem.

## Runtime Behavior Design

### Current input contract

The runtime-web host already exposes:

- `pressed`
- `just_pressed`
- `just_released`

for the jump button through the bridge and host button snapshot path.

That contract is sufficient for this slice.
No browser input contract expansion is required before implementing variable-height jump logic.

### Jump state model

The runtime core should add explicit per-player jump state rather than inferring jump phase from `vspeed` alone.

The first-state model should cover:

- whether the player is currently in an active jump phase
- how many frames the current jump has been held
- whether jump-cut behavior has already been applied for this airborne cycle
- whether the player was grounded on the prior tick

This state may live as dedicated runtime instance fields or as clearly reserved runtime variables, but it must be internal runtime state rather than a browser-shell concern.

### Required jump phases

#### A. Jump start

When the jump button is `just_pressed` and the player is grounded:

- initialize upward motion from the configured jump source
- mark the jump as active
- reset hold-frame counters
- clear any prior jump-cut flag

#### B. Hold window

While the player is still rising and the jump button remains held:

- continue the long-jump path only for the configured hold window
- stop extending jump influence once the configured hold budget is exhausted

This phase is what creates the difference between short and full jumps.

#### C. Early-release jump cut

When the player releases jump during the upward phase:

- clamp or trim upward speed once for the current airborne cycle
- do not repeatedly reapply the cut every subsequent tick

This should produce a clear short-hop outcome instead of a full jump.

#### D. Ceiling contact

When the player collides upward with a solid ceiling:

- cancel or trim upward velocity immediately
- close the active upward jump phase
- preserve downstream gravity/fall behavior normally

This must be part of the same jump model, not a separate patch on top.

#### E. Landing reset

When the player transitions from airborne to grounded:

- clear active jump state
- clear hold counters
- clear cut-applied state
- allow the next jump cycle to start cleanly

### Collision coupling

Jump behavior must remain coupled to the current axis-based collision helpers.

For this slice:

- grounded checks continue to use the runtime bbox/collision path
- ceiling checks must reflect the same solid-collision resolution path used for movement
- landing reset must occur from actual collision-backed grounded state, not only from `vspeed == 0`

This is necessary because IWanna jump feel is heavily affected by exactly when the game decides the player is grounded, airborne, or blocked above.

### Variable sourcing

The runtime should resolve jump-affecting values in this order:

1. player instance variables initialized by parsed create logic
2. stable runtime-recognized aliases already used in sample IWanna logic
3. conservative runtime defaults

The first pass should explicitly support these values when present:

- `jump`
- `jump2`
- `gravity`
- `maxFallSpeed`
- `moveSpeed` or `maxSpeed` where movement coupling matters

`djump` should be preserved and surfaced to the runtime path, but first-phase jump alignment should not expand into full multi-jump support unless `IWBT_Dife` proves it is required on the critical path.

## Architecture Changes

### `crates/iwm-runtime-core/`

Primary implementation area.

Likely touch points:

- `src/movement.rs`
  - replace fixed-height jump behavior with jump-state-driven logic
- `src/core.rs`
  - pass full jump button state into movement handling instead of only `just_pressed`
- `src/types.rs`
  - add explicit runtime jump-state fields if instance state belongs there
- `src/helpers.rs`
  - extend helper behavior only if needed for grounded / ceiling detection clarity
- `src/tests/movement.rs`
  - add deterministic jump-behavior tests
- `src/tests/support.rs`
  - extend fixtures for jump variables or trace helpers

### `crates/iwm-runtime-web/`

Expected to remain mostly unchanged because the host already carries held/pressed/released semantics.

Any changes here should be limited to:

- exposing trace or snapshot data needed for diagnostics
- keeping bridge tests aligned with unchanged input semantics

### `runtime/`

The browser shell is not where jump semantics should be implemented.

Possible shell follow-through is limited to:

- showing jump trace diagnostics when useful
- keeping browser smoke tests aligned with runtime snapshot expectations

## Trace And Validation Design

### Deterministic trace requirement

The project needs a jump-specific deterministic trace surface.

The trace should be able to capture at least:

- tick number
- input state for jump on that tick
- player `x`
- player `y`
- player `hspeed`
- player `vspeed`
- grounded state
- jump-active state
- jump-hold-frame count
- whether jump-cut has already been applied

This may be implemented as:

- a crate-local test helper
- a structured runtime diagnostic helper
- or a dedicated debug snapshot structure used only by tests

The critical requirement is deterministic comparison, not public API polish.

### Validation scenarios

The first validation set should cover:

1. tap jump from flat ground
2. held jump from flat ground
3. early release after a few rising frames
4. upward collision with a low ceiling
5. landing reset followed by a second jump
6. the same jump behavior under non-default `gravity` and `jump` values

### Gold-sample validation

`IWBT_Dife` should remain the acceptance target for this slice.

The runtime validation loop should be:

1. generate or load the local `sample/` package from `IWBT_Dife`
2. run a fixed input script against the runtime
3. capture frame traces
4. compare those traces against a checked local expectation or documented calibration target
5. refine only the runtime-core jump model and recognized variable mapping when mismatches appear

The point is to prevent "feels closer" from being the only acceptance criterion.

## Testing Strategy

### Narrow tests first

Start with `crates/iwm-runtime-core` unit/integration tests for deterministic jump behavior.

Required first-wave tests:

- fixed fixture shows shorter apex for tap than hold
- early release applies jump cut only once
- ceiling hit cancels upward travel correctly
- landing resets jump state
- runtime respects create-initialized jump variables over defaults

### Broader validation second

After core tests pass:

- run broader `iwm-runtime-core` tests
- run `cargo test` for workspace confirmation if the change surface justifies it
- run browser-facing smoke only to confirm no bridge regression

### Sample-driven validation

If a local `IWBT_Dife` package is available:

- run the jump trace against that package before claiming alignment

If it is not available in the current environment:

- keep repository tests authoritative
- mark the missing local sample trace as an environment-specific validation gap, not as proof of correctness

## Error Handling And Diagnostics

Jump alignment should stay explicit about uncertainty.

The runtime should emit diagnostics when:

- required jump variables appear to be missing and defaults are used
- a jump trace is requested but no player instance exists
- gold-sample validation cannot run because the local package artifact is absent
- the runtime hits unsupported logic on the player movement path that invalidates jump conclusions

This prevents false confidence from a partially executed sample.

## Risks

### 1. Mistaking sample script logic for runner semantics

Some behavior belongs to GM8/OpenGMK semantics, while some belongs to the game's object code.
The implementation must not flatten both into one opaque hardcoded rule.

### 2. Overfitting to one sample with the wrong abstraction

`IWBT_Dife` is the acceptance target, but the runtime should still prefer variable-driven and phase-driven rules over a one-off bespoke curve.

### 3. Grounded-state mistakes

Landing reset and repeated jump consistency depend heavily on grounded detection.
If grounded state is inferred too loosely, the jump model will still feel wrong even if hold/cut logic exists.

### 4. Event-order drift

If runtime tick order drifts from the OpenGMK-guided ordering for input, movement, and event dispatch, jump behavior can feel wrong even when per-frame speed values look plausible in isolation.

## Success Criteria

This slice is successful when all of the following are true:

- runtime-core no longer uses a fixed-height placeholder jump for the player path
- tap and hold jumps produce measurably different trajectories in deterministic tests
- ceiling and landing behavior are part of the same modeled jump cycle
- jump logic consumes held and released button semantics from the host path without shell-side hacks
- trace-based validation exists for jump behavior
- `IWBT_Dife` can serve as the acceptance target for further tuning

## Non-Goals For This Slice

Do not expand this design into:

- a full movement-engine rewrite
- full double-jump or advanced air-control mechanics unless the gold sample proves they are required immediately
- a browser-only "feel patch"
- broad package-format changes for speculative movement metadata
- replacing sample-based acceptance with intuition

## Recommendation Summary

The recommended path is:

1. keep OpenGMK as the reference for input and movement semantics
2. implement jump-state-driven behavior in `iwm-runtime-core`
3. couple jump phases to the existing collision path
4. make jump behavior variable-driven where the sample already exposes common IWanna variables
5. validate with deterministic traces
6. use `IWBT_Dife` as the first real acceptance target

This keeps the repository aligned with the Phase 4 WASM-first route while addressing one of the most player-visible fidelity gaps first.
