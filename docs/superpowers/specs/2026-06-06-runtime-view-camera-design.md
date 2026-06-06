# Runtime View And Camera Design

## Status

Approved for planning on 2026-06-06.

## Problem

Large IWBT-style rooms are currently rendered into a canvas sized to the full
room. The runtime already culls draw commands against the first visible room
view, but the submitted frame still reports `room.width` and `room.height`.

This causes rooms such as `rStage01` in the local `IWBT_Dife` package to create
a `2400x1824` canvas even though the original game displays an `800x600` view.
The result is unnecessary canvas memory use and slow browser rendering.

The original game uses two related mechanisms:

- Fixed view segments, where game logic updates `view_xview` and `view_yview`
  to move the visible source rectangle to the next screen-sized part of the
  same room.
- GM8 view following, where room view metadata names a follow target plus
  horizontal and vertical borders and speeds.

Both mechanisms originate from GM8 executable data or GML preserved in the
package. The current package already carries `source_*`, `port_*`, and `target`,
but it does not yet expose follow border and speed fields, and runtime-core does
not yet treat `view_*` variables as room view state.

## Goals

- Keep browser canvas dimensions based on the active GM8 view port, not the full
  room size.
- Render room-space draw commands through the active view transform so the
  browser canvas shows the same visible region as the original runner.
- Support the `IWBT_Dife` camera-object pattern where lowered logic writes
  `view_xview` and `view_yview` from player position.
- Extend the runtime package view contract with GM8 follow border and speed
  fields so a later follow-camera slice does not require reopening the parser
  contract.
- Keep the work in the Rust runtime/package path. Do not add new TypeScript
  gameplay heuristics.

## Non-Goals

- Full multi-view rendering parity.
- Rotated view rendering through `view_angle`.
- Draw-event parity, surfaces, or advanced blend behavior.
- Complete menu/save-select usability.
- Complete GM8 variable semantics beyond the view variables and expression
  calls required by this slice.

## Proposed Approach

Use a staged runtime-first approach.

First, make runtime-core submit frames sized to the visible view port and apply a
room-to-view translation to draw commands. This fixes the current large-canvas
performance issue even before dynamic camera movement is complete.

Second, add mutable room view state to `RuntimeRoomState`. Runtime room building
will copy the parser-provided view definitions into that state. Render code will
read the mutable state, not the immutable package room definition, when choosing
the active view.

Third, wire a narrow GM8 view variable subset into lowered-logic execution:

- `view_xview` and `view_xview[0]`
- `view_yview` and `view_yview[0]`
- `view_wview` and `view_hview` as state updates where practical, mainly to
  preserve the current sample's shake logic surface

The first implementation should target view index 0 because the gold sample uses
one visible view. Multi-view semantics can be layered later after the frame model
can describe per-port command transforms cleanly.

Fourth, add the expression support needed by the gold sample camera block:

- `floor(number)`
- object-name member reads such as `player.x` and `player.y`, resolved to the
  first alive instance whose object name matches the identifier

Finally, extend `RoomView` in the shared package model and parser export with:

- `hborder`
- `vborder`
- `hspeed`
- `vspeed`

These fields are parsed by `gm8exe` already and match OpenGMK's runtime view
state naming. Runtime follow behavior can be implemented after the fixed-segment
camera path is verified.

## Data Flow

1. Parser reads GM8 room views through `gm8exe`.
2. Package export writes all current view rectangle and port fields plus follow
   target, border, and speed fields.
3. Runtime room build copies package view definitions into mutable room state.
4. Lowered Step or room-start logic can assign `view_xview` and `view_yview`,
   updating mutable view state.
5. Render frame selection uses mutable view state to compute:
   - source rectangle in room coordinates
   - output frame size from visible port dimensions
   - draw-command translation from room coordinates to canvas coordinates
6. Browser renderer receives normal draw commands and a small frame size. It does
   not need to know room dimensions for the first single-view slice.

## Error Handling

- If views are disabled or no visible view exists, preserve current behavior and
  render the full room.
- If a view has invalid zero dimensions, fall back to the full room and emit a
  runtime diagnostic instead of panicking.
- If `view_xview` or `view_yview` receives a non-numeric value, keep the current
  view coordinate and preserve the assignment in instance variables only if that
  is already how the runtime would handle unknown fields.
- If object-name member resolution is ambiguous, use the first alive matching
  instance for this slice and keep diagnostics available for future refinement.

## Testing

Add targeted Rust tests first:

- A room with a `2400x1824` size and a visible `800x600` view submits an
  `800x600` frame.
- Tiles and sprites inside the active view are translated into canvas-space draw
  coordinates.
- Draw commands outside the active view are culled.
- A lowered Step assignment to `view_xview` and `view_yview` changes the active
  rendered region on the next frame.
- `floor(player.x / 800) * 800` evaluates through object-name member lookup.
- Parser export includes `hborder`, `vborder`, `hspeed`, and `vspeed`.

Then run the relevant broader suites:

- `cargo test`
- WASM bridge build and sync when runtime-web types change
- `npm --prefix runtime test`
- `npm --prefix runtime run test:browser` if the local browser prerequisites are
  available

## Documentation Updates

When implemented, update:

- `docs/notes/package-format-v1-runtime.md` for the expanded `RoomView`
  contract.
- `docs/notes/runtime-wasm-gap-analysis.md` to move basic view cropping and
  fixed-segment camera movement out of the missing list.
- `docs/notes/runtime-gold-sample.md` with the `IWBT_Dife` evidence that large
  rooms render through an `800x600` view.
