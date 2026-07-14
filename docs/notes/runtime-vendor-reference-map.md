# Runtime Vendor Reference Map

This note records which vendored upstream module should guide which runtime concern during the WASM-first runtime phase.

## Boundary Rules

- OpenGMK `gm8emulator` is the primary reference for GM8 runner semantics during the Beta line.
- That reference does not decide this project's package format, host-boundary shape, or licensing decisions.
- OpenGMK `gm8exe` remains the only intended direct dependency boundary for parser code, isolated behind `crates/iwm-parser/src/gm8_adapter.rs`.
- GM8Decompiler is for parser recovery behavior and odd executable comparisons only. Do not use it to define runtime semantics.

## Runtime Concern Matrix

| Concern | Vendor reference | Why it matters here | Use timing |
| --- | --- | --- | --- |
| Per-instance movement state, friction, gravity, speed application, and bounce/collision response | `vendor/OpenGMK/gm8emulator/src/game/movement.rs` | The Beta runtime needs a vendor-guided baseline for player motion, gravity, and solid-contact behavior instead of re-inventing GM8-like rules in the WASM core | Now |
| Event ownership and ordering for object, keyboard, mouse, trigger, alarm, and shutdown flows | `vendor/OpenGMK/gm8emulator/src/game/events.rs` | The runtime core needs a reliable reference for when events run, which instances receive them, and when pending room changes suppress further event execution | Now |
| Instance-variable owners combined with array accessors | `vendor/OpenGMK/gm8emulator/src/gml/compiler.rs` and `vendor/OpenGMK/gm8emulator/src/gml/runtime.rs` (`FieldAccessor`, `VariableAccessor`, `SetField`, and `SetVariable`) | GM8 keeps the receiver target separate from the array index; object-owner writes apply to matching instances instead of flattening the owner into a local variable name. This is the reference for expressions such as `timelimitobject.alarm[0]` | Now |
| Timeline advancement and crossed-moment ordering | `vendor/OpenGMK/gm8emulator/src/game.rs` (timeline advancement in the main step path) | Per-instance timeline position, speed, loop behavior, and forward/reverse moment ranges drive scripted endurance-room phases | Now |
| DnD action semantics for timelines, motion creation, Dice, sprite transforms, sound, and wrapping | `vendor/OpenGMK/gm8emulator/src/gml/kernel.rs` | Parser lowering and runtime helpers must agree on argument order, relative flags, and action-specific state changes instead of flattening the action list | Now |
| Outside/intersect boundary event tests | `vendor/OpenGMK/gm8emulator/src/game/events.rs` (`run_bound_events`) | Moving bullets must receive `other:outside` only after their bounds leave the room, while wrapping objects can handle that same event without being destroyed | Now |
| Semantic room restart and room-switch behavior | `vendor/OpenGMK/gm8emulator/src/game/transition.rs` | Room restart and room change are on the critical gameplay path, so this file matters immediately as a reference for scene-change behavior | Now |
| Presentation-specific room transition effects | `vendor/OpenGMK/gm8emulator/src/game/transition.rs` | The same file also defines wipes, fades, slides, and user transition hooks, but those visual effects can wait until semantic room switching is stable | Later |
| GameMaker-facing keyboard and mouse state model, including held/pressed/released queries and per-frame reset rules | `vendor/OpenGMK/gm8emulator/src/input.rs` | The browser shell and WASM bridge need to map web input into GM8-style button state transitions without drifting from expected `keyboard_*` and `mouse_*` behavior; one-shot edges belong in the host/input adapter, not in shell gameplay code | Now |
| Renderer abstraction, sprite draw entrypoints, view/projection setup, and frame presentation shape | `vendor/OpenGMK/gm8emulator/src/render.rs` | The project already has its own web drawing path, but this file defines what the runtime eventually expects a host renderer to provide and is the reference for draw-surface parity checks | Later |
| Parser recovery orientation and weird executable comparison cases | `vendor/GM8Decompiler/README.org` | This README is only a high-level orientation source for how GM8Decompiler approaches executable-to-project recovery; use it to frame validation work on odd samples, not as a code-level implementation guide | Only for validation |

## Practical Reading Order

1. Use `movement.rs`, `events.rs`, and `input.rs` first for the current runtime-core semantics slice.
2. Use `transition.rs` immediately for room restart and room-switch semantics; use the visual transition portions later once those semantics are stable.
3. Use `render.rs` to audit host-surface expectations and draw-order gaps, not to reopen a parallel TypeScript gameplay runtime.
4. Use GM8Decompiler only when parser output on unusual executables needs comparison or recovery-oriented sanity checks.
