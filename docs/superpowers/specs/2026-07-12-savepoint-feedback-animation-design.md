# SavePoint Feedback Animation Design

## Status

Approved for design documentation on 2026-07-12.

## Problem

In the local `IWBT_Dife` gold sample, activating a `savePoint` with the `S` key
correctly saves the game and removes the checkpoint, but its feedback animation
does not match GM8:

- the seven `object809` bubble particles become invisible immediately instead
  of moving outward and fading over roughly 50 steps
- the `object808` full-view white overlay is not reliably visible, especially
  when the active view is offset from room origin

The parser already preserves and lowers the relevant GML. The failure is in
runtime consumption of GM built-in instance and view variables, plus incomplete
instance draw ordering.

## Source Behavior

The activated `savePoint` creates:

- one `object808` at `view_xview[0], view_yview[0]`
- seven `object809` instances at the checkpoint position
- one invisible `object819` helper that recreates the checkpoint after its
  alarm expires

`object808` starts with `image_alpha = 0.7` and subtracts `0.01` each Step.
Its sprite is an `800x608` white image intended to cover the active view.

`object809` assigns random `speed` and `direction`, then subtracts `0.02` from
`image_alpha` each Step. In GM8, an instance starts with `image_alpha = 1`, so
the bubbles remain visible while moving outward and gradually fading.

## Root Causes

### Missing built-in instance defaults during expression evaluation

Rendering treats a missing `image_alpha` as `1`, but lowered expression
evaluation does not. The first `image_alpha -= 0.02` on `object809` therefore
operates on an unresolved value instead of the GM8 default. The stored result
becomes non-positive and the renderer clamps it to transparent.

### Indexed view reads do not use live room view state

Lowered `view_xview[0]` and `view_yview[0]` reads only search scope, globals, and
instance variables. They do not read `RuntimeRoomState.views[0]`. When the
values cannot be resolved, `instance_create()` silently uses zero coordinates.
The white overlay is consequently created at room `(0, 0)` and can be culled
when the active view is elsewhere.

### Instance rendering does not honor GM depth

Runtime instance sprites are emitted in room instance-vector order. GM8 draws
larger depth values first and smaller depth values later. `object808` has depth
`-999999999`, explicitly making it a frontmost overlay, but the current renderer
does not guarantee that ordering.

## Chosen Approach

Implement general GM8 runtime semantics rather than Dife-specific rules.

### Built-in instance variable reads

Add a shared built-in instance-variable lookup used when no explicit instance
variable exists. The first slice must cover:

- `image_alpha = 1`
- `image_xscale = 1`
- `image_yscale = 1`
- `image_index = 0`
- `image_speed = 1`
- `visible = true`

Explicit assignments remain authoritative. Defaults should be exposed through
normal expression evaluation so compound assignments behave like GM8. The
implementation does not need to eagerly duplicate every default into the
instance variable map.

### Live view-variable reads

Extend runtime evaluation context with the active room view values needed by
lowered expressions. Support both scalar and indexed spellings for view zero:

- `view_xview` and `view_xview[0]`
- `view_yview` and `view_yview[0]`
- `view_wview` and `view_wview[0]`
- `view_hview` and `view_hview[0]`

The values must reflect mutable runtime view state after camera logic runs, not
only immutable package metadata. Existing view assignments continue to update
the shared room view state.

`instance_create()` should consume these resolved values. The general fallback
behavior for truly unresolved numeric arguments can remain compatible for this
slice, but the Dife view expressions must no longer reach that fallback.

### Stable GM depth ordering

Before emitting ordinary instance sprite commands, collect live visible
instances and sort them stably by depth descending. This produces GM ordering:

- larger depth values render earlier and farther back
- smaller depth values render later and farther forward
- equal-depth instances retain their existing relative order

Draw-event commands and foreground room backgrounds keep their existing phases
unless tests show a separate GM ordering incompatibility. This change is scoped
to the ordinary instance sprite ordering required by the feedback overlay.

## Data Flow

1. Parser emits the existing lowered savePoint, `object808`, and `object809`
   blocks without sample-specific transformation.
2. Runtime collision dispatch executes the savePoint block and creates the
   three feedback object types.
3. Runtime expression evaluation resolves indexed view coordinates from the
   active mutable room view.
4. Newly created bubbles read the GM default `image_alpha = 1` before applying
   their first subtraction.
5. Existing `speed` and `direction` assignment handling updates horizontal and
   vertical motion components.
6. Each tick advances bubble position and alpha.
7. Render-frame construction sorts ordinary visible instances by GM depth and
   emits alpha-bearing sprite commands.
8. The browser renderer uses the existing draw-command alpha support.

## Error Handling And Diagnostics

- Explicit non-numeric values assigned to numeric built-ins should retain the
  existing unsupported or fallback behavior rather than being silently replaced
  by defaults.
- Missing view zero should fall back to the current no-view/full-room behavior;
  it must not panic.
- Multi-view behavior beyond view zero is out of scope.
- No Dife object ids or object names should appear in production runtime logic.

## Testing

Add narrow runtime-core tests before implementation:

- a compound assignment reads the default `image_alpha` as `1`
- explicit `image_alpha` overrides the default and reaches draw commands
- indexed view reads resolve the active mutable view coordinates
- an instance created at `view_xview[0], view_yview[0]` renders at canvas
  `(0, 0)` when the view has a nonzero room offset
- instance sprite commands are stably ordered by depth descending

Extend the feature-gated real-sample savePoint regression to verify:

- exactly seven live `object809` particles are created
- their first post-Step alpha is near `0.98`, not zero
- their positions diverge from the savePoint over subsequent ticks
- their alpha decreases over several sampled ticks and they remain visible for
  most of their intended lifetime
- `object808` emits sprite 524 at canvas `(0, 0)` with alpha near `0.7` in an
  offset-view room
- the overlay alpha decreases over subsequent ticks
- the overlay draw command appears after normal-depth instance sprites
- the existing `object819` checkpoint respawn behavior remains intact

Run verification in increasing scope:

1. targeted runtime-core unit tests
2. the feature-gated Dife savePoint regression
3. `cargo test`
4. `npm --prefix runtime test`
5. release WASM build, sync, and browser smoke when local prerequisites are
   available

## Documentation Updates

After implementation, update:

- `docs/notes/runtime-wasm-gap-analysis.md` with built-in alpha, live indexed
  view-read, and depth-order support
- `docs/notes/runtime-gold-sample.md` with the verified Dife savePoint feedback
  behavior

The package format does not change, so `docs/notes/package-format-v1-runtime.md`
does not require an update unless implementation reveals a contract change.

## Non-Goals

- complete GM8 built-in variable coverage
- arbitrary multi-view expression semantics
- advanced blend modes or surfaces
- a sample-specific particle or flash implementation
- unrelated draw-event ordering refactors
