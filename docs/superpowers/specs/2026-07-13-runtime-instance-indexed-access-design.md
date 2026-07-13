# Runtime Instance Indexed Access Design

## Problem

ArioTrials room 156 starts timeline 18 and renders the endurance barrage, but its
visible time limit remains at `1:55`. Timeline moment 1 contains the valid GM8
assignment `timelimitobject.alarm[0] = room_speed`. The parser preserves that
assignment as an `IndexAccess` whose base is a `MemberAccess`, but the runtime
only recognizes a top-level `MemberAccess` as a cross-instance assignment.

The runtime therefore flattens the full target to the string key
`timelimitobject.alarm[0]` and stores it on the timeline owner. It never writes
`alarm[0]` on the live `timelimitobject` instance, so its alarm event cannot
start. This produces no unsupported diagnostic because both the expression and
the fallback local assignment are otherwise valid.

## Chosen Design

Represent the assignment target as two logical parts before dispatch:

- an optional instance receiver such as an object name, `self`, `other`, or an
  evaluated instance reference
- a member key that retains any array suffix, such as `alarm[0]` or `values[3]`

The existing cross-instance update path will receive that structured result and
write the member key through `RuntimeSparseInstanceOverlay`. Targets without an
instance receiver, including `alarm[0]` and `global.values[3]`, continue through
the existing local/global assignment path.

Object-name receivers follow GM8 owner semantics: writes apply to every live
instance matching the object or its descendants, while reads use the first live
matching instance. `self`, `other`, and evaluated instance references select one
instance. Pending instance creations remain supported by the same receiver
forms where the current runtime can resolve them.

## Runtime Changes

`crates/iwm-runtime-core/src/logic/instances.rs` will own receiver-aware indexed
member resolution. It will:

1. recognize both `receiver.member` and `receiver.member[index]`
2. evaluate the index with the existing canonical key formatting
3. resolve object receivers to all matching live instance indices
4. route each write through the existing sparse overlay
5. avoid writing the flattened receiver name onto the event owner

`crates/iwm-runtime-core/src/logic/eval_variables.rs` will mirror the target
decomposition for indexed reads so read and write semantics do not diverge.
Parser and package schemas remain unchanged because the lowered IR is already
correct.

## Compatibility Boundaries

- This change does not special-case ArioTrials, timelines, or alarm slot zero.
- One-dimensional indexed members use the runtime's existing flattened key
  representation, for example `alarm[0]`.
- Unsupported multidimensional or dynamically unresolvable targets retain
  existing behavior; expanding those is outside this fix.
- Timeline ordering, alarm decrement ordering, and browser rendering do not
  change.

## Verification

Runtime-core regression tests will prove that:

- a timeline can assign `timer.alarm[0]`
- the target alarm counts down and dispatches its event
- the timeline owner never receives `timer.alarm[0]`
- an object-name indexed assignment updates every matching live instance
- indexed reads observe receiver-targeted values

The targeted runtime-core tests will be run red before implementation and green
afterward. Then the runtime-core crate, workspace Rust suite, ArioTrials package
validation/diagnostics, and graph update will be run. Current runtime notes will
be updated to record the corrected GM8 owner-plus-array-accessor semantics.
