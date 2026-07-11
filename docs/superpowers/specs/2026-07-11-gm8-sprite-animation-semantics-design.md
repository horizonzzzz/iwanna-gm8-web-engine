# GM8 Sprite Animation Semantics Design

Date: 2026-07-11

## Goal

Implement the GM8 runtime semantics needed for object logic such as:

```gml
sprite_index = sprPlayerRunning;
image_speed = 0.5;
```

Sprite resource identifiers must work as general GM constants. The implementation
must not special-case Kid objects, IWanna object names, Dife resource names, or
specific numeric resource IDs.

## Current Gap

The parser already lowers named sprite assignments without losing their source
meaning. For example, the Dife player Step event contains lowered assignments to
`sprPlayerRunning`, `sprPlayerJump`, and `sprPlayerFall`.

Runtime expression evaluation currently resolves local, instance, global, key,
room, and object identifiers, but it does not resolve sprite resource names. The
right-hand side of a named `sprite_index` assignment therefore produces no value,
and the assignment is skipped. The instance remains on its object-default sprite,
even though sprite frame advancement and rendering already support `image_speed`
and `image_index`.

## Runtime Semantics

### Resource Constant Resolution

Runtime expression evaluation will resolve a sprite resource name to its numeric
sprite ID after normal variable and existing constant lookup has failed. This
preserves variable shadowing while making sprite names available anywhere a GM
numeric resource constant is valid, rather than only inside `sprite_index`
assignments.

The lookup is case-insensitive, matching the runtime's existing named-resource
indexes.

### Sprite Switching

Writing `sprite_index` will use GM8/OpenGMK-compatible instance behavior:

- update the instance sprite ID when the value changes;
- preserve the current fractional `image_index` when it is valid for the new
  sprite;
- reset `image_index` to zero when `floor(image_index)` is outside the new
  sprite's frame range;
- leave `image_speed` unchanged;
- continue advancing animation through the existing per-tick animation phase;
- continue rendering `floor(image_index)`.

This keeps animation continuity for equal-sized sprite changes while preventing
invalid frames when changing to a shorter animation.

## Data Flow

1. Parser lowering retains a sprite name as a structured identifier expression.
2. Runtime expression evaluation checks locals, instance variables, globals, and
   existing GM constants.
3. If unresolved, it checks the package sprite-name index and returns the sprite
   ID as a numeric runtime value.
4. Instance assignment applies sprite-switch normalization using the package's
   sprite frame count.
5. The existing animation phase advances `image_index` by `image_speed` and the
   renderer selects `floor(image_index)`.

## Testing

Runtime-core tests will cover:

- a named sprite constant used by a lowered assignment resolves to the package
  sprite ID;
- a local or instance variable with the same name continues to shadow the sprite
  constant;
- switching between animations preserves a valid fractional `image_index`;
- switching to a shorter animation resets an out-of-range `image_index`;
- `image_speed` continues advancing the selected sprite after a switch;
- the local Dife sample selects running, jumping, and falling sprites through its
  existing parsed Player Step GML and advances their frames.

The narrow runtime-core tests run first, followed by workspace Rust tests. The
generated local sample remains ignored and is used only when present.

## Documentation Impact

Update `docs/notes/runtime-wasm-gap-analysis.md` to record that named sprite GM
constants and OpenGMK-compatible sprite switching are supported. Update the gold
sample note with the verified Kid animation behavior when the local Dife package
test passes.

## Non-Goals

- hardcoded IWanna animation state machines;
- parser rewriting of sprite names into sample-specific numeric IDs;
- adding new sprite assets or editing exported frames;
- full support for every GM8 asset constant category in the same change;
- changing collision-mask frame selection beyond the existing runtime behavior.
