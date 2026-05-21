# WASM Browser Input And Render Loop Implementation Plan

> **Status note:** Historical implementation plan.
>
> This document captures an intermediate runtime step and should be read as project history unless its tasks still match current repository reality.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `iwm-runtime-web` usable as a real browser runtime path by wiring browser keyboard input into the WASM host and exposing a browser-consumable frame surface that the frontend shell can draw on a canvas.

**Architecture:** Keep the current JSON-over-linear-memory bridge. Extend the Rust host/core path to emit stable draw commands and accept explicit input snapshots. Keep browser-specific canvas/image concerns in `runtime/` by translating bridge frames into real canvas draws using the existing package resource index. Reuse the current fixed-step loop as the browser clock so the TS shell and WASM path share the same outer control model.

**Tech Stack:** Rust 1.77+, Cargo workspace, `wasm32-unknown-unknown`, TypeScript, Vite, Vitest, existing `runtime/` shell and canvas renderer

---

## Scope

In scope for this slice:

- keyboard input for left/right/jump/restart
- render-command emission from the Rust runtime path
- frontend rendering of WASM frames on the existing canvas
- pause/resume/reset behavior for the WASM runtime path
- one explicit local smoke path for a normalized package in the browser shell

Out of scope for this slice:

- audio playback
- mouse input
- DLL/external compatibility
- surfaces, particles, or advanced draw APIs
- replacing the existing parser/package contract
- full OpenGMK gameplay-fidelity claims

## File Structure

Planned files for this phase:

- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Modify: `crates/iwm-runtime-core/src/lib.rs`
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Create: `runtime/src/runtime/wasmSession.ts`
- Create: `runtime/src/runtime/wasmSession.test.ts`
- Create: `runtime/src/render/wasmFrameRenderer.ts`
- Create: `runtime/src/render/wasmFrameRenderer.test.ts`
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`
- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-gold-sample.md`

Responsibilities:

- `iwm-runtime-host`: stable host-facing input and frame contracts
- `iwm-runtime-core`: deterministic room-state-to-frame conversion plus input-driven tick behavior
- `iwm-runtime-web`: JSON bridge API for input submission and frame retrieval
- `wasmBridge.ts`: browser-side ABI wrapper and TypeScript bridge types
- `wasmSession.ts`: fixed-step browser driver for the WASM runtime path
- `wasmFrameRenderer.ts`: canvas renderer for bridge frame commands
- `shell.ts`: package load flow, runtime controls, and frontend wiring
- docs: record the updated browser usage and smoke path

## Preconditions

Before starting this phase:

- `cargo test` should pass on `master`
- `npm --prefix runtime test` should pass
- `npm --prefix runtime run build` should pass
- `cargo build -p iwm-runtime-web --target wasm32-unknown-unknown` should already work in a Windows developer shell with `clang` configured
- `npm --prefix runtime run sync:wasm` should already copy the local artifact into `runtime/public/wasm/`

### Task 1: Extend The Rust Host And Core Frame Contracts

**Files:**
- Modify: `crates/iwm-runtime-host/src/lib.rs`
- Modify: `crates/iwm-runtime-core/src/lib.rs`

- [ ] **Step 1: Write the failing Rust tests for richer draw commands and input snapshots**

Add a new unit test in `crates/iwm-runtime-core/src/lib.rs` that proves the runtime can emit a frame containing background, sprite, and fallback-rect commands from the current room state.

```rust
#[test]
fn runtime_core_emits_browser_consumable_draw_commands() {
    let package = sample_package();
    let mut core = RuntimeCore::load(package).unwrap();
    let mut host = HeadlessHost::new("runtime-core");

    core.render(&mut host).unwrap();

    let frame = host.renderer.submitted_frames.last().unwrap();
    assert_eq!(frame.room_id, Some(7));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawBackground { background_id: 0, .. }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::DrawSprite { sprite_id: 0, .. }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        RuntimeDrawCommand::FillRect { .. }
    )));
}
```

Also extend the existing host tests in `crates/iwm-runtime-host/src/lib.rs` so `SnapshotInputHost` can replace the full button-state set in one call and `NullRenderHost` keeps the last submitted frame shape.

```rust
#[test]
fn snapshot_input_host_replaces_button_states() {
    let mut input = SnapshotInputHost::default();
    input.replace_button_states([
        (
            RuntimeButton::Keyboard(0x25),
            ButtonState {
                pressed: true,
                just_pressed: true,
                just_released: false,
            },
        ),
    ]);

    assert!(input.button_state(RuntimeButton::Keyboard(0x25)).pressed);
    assert!(!input.button_state(RuntimeButton::Keyboard(0x27)).pressed);
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail**

Run:

```powershell
cargo test -p iwm-runtime-core runtime_core_emits_browser_consumable_draw_commands
cargo test -p iwm-runtime-host snapshot_input_host_replaces_button_states
```

Expected:

```text
error[E0599]: no method named `render` found for struct `RuntimeCore`
error[E0599]: no method named `replace_button_states` found for struct `SnapshotInputHost`
```

- [ ] **Step 3: Implement the minimal frame and input surface in Rust**

In `crates/iwm-runtime-host/src/lib.rs`, expand the render command model and add bulk input replacement helpers.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeDrawCommand {
    Clear { colour: Rgba8 },
    DrawBackground {
        background_id: usize,
        x: i32,
        y: i32,
        stretch: bool,
        tile_horz: bool,
        tile_vert: bool,
        is_foreground: bool,
    },
    DrawSprite {
        sprite_id: usize,
        frame_index: usize,
        x: i32,
        y: i32,
        origin_x: i32,
        origin_y: i32,
        xscale: f64,
        yscale: f64,
        angle_degrees: f64,
    },
    FillRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        colour: Rgba8,
    },
    Present,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRenderFrame {
    pub tick: u64,
    pub room_id: Option<usize>,
    pub width: u32,
    pub height: u32,
    pub commands: Vec<RuntimeDrawCommand>,
}

impl SnapshotInputHost {
    pub fn replace_button_states(
        &mut self,
        states: impl IntoIterator<Item = (RuntimeButton, ButtonState)>,
    ) {
        self.buttons.clear();
        self.buttons.extend(states);
    }

    pub fn clear_transitions(&mut self) {
        for state in self.buttons.values_mut() {
            state.just_pressed = false;
            state.just_released = false;
        }
    }
}
```

In `crates/iwm-runtime-core/src/lib.rs`, add a pure room-to-frame conversion path and call it from a new `render` method.

```rust
pub fn render<H: RuntimeHost>(&mut self, host: &mut H) -> Result<(), RuntimeCoreError> {
    let frame = self.build_render_frame()?;
    host.submit_frame(frame)?;
    Ok(())
}

fn build_render_frame(&self) -> Result<RuntimeRenderFrame, RuntimeCoreError> {
    let room = self.current_room.as_ref().ok_or(RuntimeCoreError::NoRooms)?;
    let source_room = self
        .room_index
        .get(&room.room_id)
        .and_then(|index| self.package.rooms.get(*index))
        .ok_or(RuntimeCoreError::RoomMissing(room.room_id))?;

    let mut commands = vec![RuntimeDrawCommand::Clear {
        colour: Rgba8 {
            r: 12,
            g: 16,
            b: 22,
            a: 255,
        },
    }];

    commands.extend(source_room.backgrounds.iter().filter(|layer| {
        layer.visible_on_start && !layer.is_foreground && layer.source_bg >= 0
    }).map(|layer| RuntimeDrawCommand::DrawBackground {
        background_id: layer.source_bg as usize,
        x: layer.xoffset,
        y: layer.yoffset,
        stretch: layer.stretch,
        tile_horz: layer.tile_horz,
        tile_vert: layer.tile_vert,
        is_foreground: false,
    }));

    for instance in &room.instances {
        if let Some(object) = self.package.objects.get(instance.object_id) {
            if object.visible && object.sprite_index >= 0 {
                commands.push(RuntimeDrawCommand::DrawSprite {
                    sprite_id: object.sprite_index as usize,
                    frame_index: 0,
                    x: instance.x,
                    y: instance.y,
                    origin_x: 0,
                    origin_y: 0,
                    xscale: 1.0,
                    yscale: 1.0,
                    angle_degrees: 0.0,
                });
                continue;
            }
        }

        commands.push(RuntimeDrawCommand::FillRect {
            x: instance.x - 4,
            y: instance.y - 4,
            width: 8,
            height: 8,
            colour: Rgba8 {
                r: 96,
                g: 112,
                b: 138,
                a: 255,
            },
        });
    }

    commands.extend(source_room.backgrounds.iter().filter(|layer| {
        layer.visible_on_start && layer.is_foreground && layer.source_bg >= 0
    }).map(|layer| RuntimeDrawCommand::DrawBackground {
        background_id: layer.source_bg as usize,
        x: layer.xoffset,
        y: layer.yoffset,
        stretch: layer.stretch,
        tile_horz: layer.tile_horz,
        tile_vert: layer.tile_vert,
        is_foreground: true,
    }));

    commands.push(RuntimeDrawCommand::Present);

    Ok(RuntimeRenderFrame {
        tick: self.tick,
        room_id: Some(room.room_id),
        width: room.width,
        height: room.height,
        commands,
    })
}
```

Update `tick` so it calls `self.render(host)?;` instead of hand-building the placeholder clear/present pair. Preserve the current diagnostics behavior.

- [ ] **Step 4: Run the targeted Rust tests to verify the new frame contract passes**

Run:

```powershell
cargo test -p iwm-runtime-host
cargo test -p iwm-runtime-core
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit**

```powershell
git add crates/iwm-runtime-host/src/lib.rs crates/iwm-runtime-core/src/lib.rs
git commit -m "feat: add wasm runtime frame and input contracts"
```

### Task 2: Expose Input Submission And Frame Retrieval Through The WASM Bridge

**Files:**
- Modify: `crates/iwm-runtime-web/src/lib.rs`
- Modify: `runtime/src/runtime/wasmBridge.ts`
- Modify: `runtime/src/runtime/wasmBridge.test.ts`

- [ ] **Step 1: Write the failing bridge tests for input submission and frame retrieval**

In `crates/iwm-runtime-web/src/lib.rs`, add a unit test that proves the web host accepts an input payload, advances one tick, and returns the latest render frame as JSON.

```rust
#[test]
fn web_runtime_host_accepts_input_and_returns_render_frame_json() {
    let mut host = WebRuntimeHost::new();
    host.boot(sample_package()).unwrap();

    host.set_input(WebInputState {
        left: true,
        right: false,
        jump: true,
        jump_pressed: true,
        jump_released: false,
        restart: false,
    });

    host.tick(1).unwrap();
    let frame = host.frame_snapshot().unwrap();

    assert_eq!(frame.tick, 1);
    assert_eq!(frame.room_id, Some(0));
    assert!(frame.commands.iter().any(|command| command.contains("drawSprite")));
}
```

In `runtime/src/runtime/wasmBridge.test.ts`, add a test that expects the ABI wrapper to expose `setInput()` and `frame()`.

```ts
it('wraps input submission and frame snapshot exports', async () => {
  const encodedFrame = new TextEncoder().encode(
    JSON.stringify({
      tick: 1,
      roomId: 0,
      width: 320,
      height: 240,
      commands: [{ kind: 'present' }]
    })
  );

  // existing memory setup omitted for brevity

  const bridge = makeWasmRuntimeBridge({
    memory,
    iwm_alloc: () => 8,
    iwm_free: () => undefined,
    iwm_boot_json: () => snapshotPointer,
    iwm_tick: () => snapshotPointer,
    iwm_reset: () => snapshotPointer,
    iwm_select_room: () => snapshotPointer,
    iwm_snapshot_json: () => snapshotPointer,
    iwm_diagnostics_json: () => diagnosticsPointer,
    iwm_set_input_json: () => snapshotPointer,
    iwm_frame_json: () => framePointer,
    iwm_last_result_len: () => lastResultLength,
  });

  await bridge.setInput({
    left: true,
    right: false,
    jump: true,
    jumpPressed: true,
    jumpReleased: false,
    restart: false,
  });

  expect((await bridge.frame()).tick).toBe(1);
});
```

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```powershell
cargo test -p iwm-runtime-web web_runtime_host_accepts_input_and_returns_render_frame_json
npm --prefix runtime test -- wasmBridge.test.ts
```

Expected:

```text
error[E0599]: no method named `set_input` found for struct `WebRuntimeHost`
Property 'setInput' does not exist on type 'WasmRuntimeBridge'
```

- [ ] **Step 3: Implement the minimal bridge API additions**

In `crates/iwm-runtime-web/src/lib.rs`, add a JSON-friendly input model plus frame snapshot types.

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebInputState {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub jump_pressed: bool,
    pub jump_released: bool,
    pub restart: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum BridgeDrawCommand {
    Clear { colour: [u8; 4] },
    DrawBackground {
        background_id: usize,
        x: i32,
        y: i32,
        stretch: bool,
        tile_horz: bool,
        tile_vert: bool,
        is_foreground: bool,
    },
    DrawSprite {
        sprite_id: usize,
        frame_index: usize,
        x: i32,
        y: i32,
        origin_x: i32,
        origin_y: i32,
        xscale: f64,
        yscale: f64,
        angle_degrees: f64,
    },
    FillRect {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        colour: [u8; 4],
    },
    Present,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeFrameSnapshot {
    pub tick: u64,
    pub room_id: Option<usize>,
    pub width: u32,
    pub height: u32,
    pub commands: Vec<BridgeDrawCommand>,
}
```

Add `set_input` and `frame_snapshot` methods on `WebRuntimeHost`, using `host.input.replace_button_states(...)` and the last submitted `RuntimeRenderFrame`.

```rust
pub fn set_input(&mut self, input: WebInputState) {
    self.host.input.replace_button_states([
        (
            RuntimeButton::Keyboard(0x25),
            ButtonState {
                pressed: input.left,
                just_pressed: input.left,
                just_released: false,
            },
        ),
        (
            RuntimeButton::Keyboard(0x27),
            ButtonState {
                pressed: input.right,
                just_pressed: input.right,
                just_released: false,
            },
        ),
        (
            RuntimeButton::Keyboard(0x20),
            ButtonState {
                pressed: input.jump,
                just_pressed: input.jump_pressed,
                just_released: input.jump_released,
            },
        ),
        (
            RuntimeButton::Keyboard(0x52),
            ButtonState {
                pressed: input.restart,
                just_pressed: input.restart,
                just_released: false,
            },
        ),
    ]);
}

pub fn frame_snapshot(&self) -> Result<BridgeFrameSnapshot, String> {
    let frame = self
        .host
        .renderer
        .submitted_frames
        .last()
        .ok_or_else(|| "runtime has not submitted a frame yet".to_string())?;

    Ok(BridgeFrameSnapshot {
        tick: frame.tick,
        room_id: frame.room_id,
        width: frame.width,
        height: frame.height,
        commands: frame.commands.iter().map(bridge_draw_command).collect(),
    })
}
```

Export two new C ABI functions:

```rust
#[no_mangle]
pub extern "C" fn iwm_set_input_json(pointer: *const u8, len: usize) -> usize

#[no_mangle]
pub extern "C" fn iwm_frame_json() -> usize
```

In `runtime/src/runtime/wasmBridge.ts`, extend the TypeScript bridge contract.

```ts
export type WasmRuntimeInputState = {
  left: boolean;
  right: boolean;
  jump: boolean;
  jumpPressed: boolean;
  jumpReleased: boolean;
  restart: boolean;
};

export type WasmRuntimeFrame = {
  tick: number;
  roomId: number | null;
  width: number;
  height: number;
  commands: Array<
    | { kind: 'clear'; colour: [number, number, number, number] }
    | { kind: 'drawBackground'; backgroundId: number; x: number; y: number; stretch: boolean; tileHorz: boolean; tileVert: boolean; isForeground: boolean }
    | { kind: 'drawSprite'; spriteId: number; frameIndex: number; x: number; y: number; originX: number; originY: number; xscale: number; yscale: number; angleDegrees: number }
    | { kind: 'fillRect'; x: number; y: number; width: number; height: number; colour: [number, number, number, number] }
    | { kind: 'present' }
  >;
};

export type WasmRuntimeBridge = {
  backend: 'opengmk-wasm';
  boot: (pkg: RuntimePackage) => Promise<WasmRuntimeBridgeSnapshot>;
  snapshot: () => Promise<WasmRuntimeBridgeSnapshot>;
  frame: () => Promise<WasmRuntimeFrame>;
  setInput: (input: WasmRuntimeInputState) => Promise<WasmRuntimeBridgeSnapshot>;
  tick: (frames?: number) => Promise<WasmRuntimeBridgeSnapshot>;
  reset: () => Promise<WasmRuntimeBridgeSnapshot>;
  selectRoom: (roomId: number) => Promise<WasmRuntimeBridgeSnapshot>;
  diagnostics: () => Promise<string[]>;
};
```

Update the ABI exports guard and wrapper methods accordingly.

```ts
type WasmRuntimeExports = {
  memory: { buffer: ArrayBufferLike };
  iwm_alloc: (size: number) => number;
  iwm_free: (pointer: number, size: number) => void;
  iwm_boot_json: (pointer: number, size: number) => number;
  iwm_set_input_json: (pointer: number, size: number) => number;
  iwm_tick: (frames: number) => number;
  iwm_reset: () => number;
  iwm_select_room: (roomId: number) => number;
  iwm_snapshot_json: () => number;
  iwm_frame_json: () => number;
  iwm_diagnostics_json: () => number;
  iwm_last_result_len: () => number;
};
```

- [ ] **Step 4: Run the targeted bridge tests**

Run:

```powershell
cargo test -p iwm-runtime-web
npm --prefix runtime test -- wasmBridge.test.ts
```

Expected:

```text
test result: ok
```

- [ ] **Step 5: Commit**

```powershell
git add crates/iwm-runtime-web/src/lib.rs runtime/src/runtime/wasmBridge.ts runtime/src/runtime/wasmBridge.test.ts
git commit -m "feat: expose wasm runtime input and frame bridge"
```

### Task 3: Add A Browser WASM Session Driver And Canvas Frame Renderer

**Files:**
- Create: `runtime/src/runtime/wasmSession.ts`
- Create: `runtime/src/runtime/wasmSession.test.ts`
- Create: `runtime/src/render/wasmFrameRenderer.ts`
- Create: `runtime/src/render/wasmFrameRenderer.test.ts`

- [ ] **Step 1: Write the failing frontend tests for the session loop and frame renderer**

Create `runtime/src/runtime/wasmSession.test.ts` to prove the browser session uses `FixedStepLoop`, pushes input before each tick, and requests a fresh frame after ticking.

```ts
import { describe, expect, it, vi } from 'vitest';
import { WasmRuntimeSession } from './wasmSession';

describe('WasmRuntimeSession', () => {
  it('submits input and fetches a frame for each manual step', async () => {
    const bridge = {
      backend: 'opengmk-wasm' as const,
      boot: vi.fn(),
      snapshot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
      frame: vi.fn(async () => ({ tick: 1, roomId: 0, width: 320, height: 240, commands: [{ kind: 'present' as const }] })),
      setInput: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
      tick: vi.fn(async () => ({ tick: 1, roomId: 0, diagnostics: [] })),
      reset: vi.fn(),
      selectRoom: vi.fn(),
      diagnostics: vi.fn(async () => []),
    };

    const session = new WasmRuntimeSession(bridge);
    session.setInputState({ left: true, right: false, jump: false, restart: false });

    const frame = await session.stepOnce();

    expect(bridge.setInput).toHaveBeenCalled();
    expect(bridge.tick).toHaveBeenCalledWith(1);
    expect(frame.width).toBe(320);
  });
});
```

Create `runtime/src/render/wasmFrameRenderer.test.ts` to prove bridge frames can be drawn through a fake `CanvasRenderingContext2D`.

```ts
import { describe, expect, it } from 'vitest';
import { renderWasmFrame } from './wasmFrameRenderer';

describe('renderWasmFrame', () => {
  it('draws clear, rect, and present commands', async () => {
    const calls: string[] = [];
    const context = {
      clearRect: () => calls.push('clearRect'),
      fillRect: () => calls.push('fillRect'),
      drawImage: () => calls.push('drawImage'),
      save: () => calls.push('save'),
      restore: () => calls.push('restore'),
      translate: () => calls.push('translate'),
      rotate: () => calls.push('rotate'),
      scale: () => calls.push('scale'),
      set fillStyle(_value: string) {
        calls.push('fillStyle');
      },
    };

    const canvas = {
      width: 0,
      height: 0,
      getContext: () => context,
    } as unknown as HTMLCanvasElement;

    await renderWasmFrame(canvas, sampleFrame, samplePackage.resources, '/packages/sample');

    expect(calls).toContain('fillRect');
  });
});
```

- [ ] **Step 2: Run the targeted frontend tests to verify they fail**

Run:

```powershell
npm --prefix runtime test -- wasmSession.test.ts
npm --prefix runtime test -- wasmFrameRenderer.test.ts
```

Expected:

```text
Failed to resolve import "./wasmSession"
Failed to resolve import "./wasmFrameRenderer"
```

- [ ] **Step 3: Implement the browser WASM session driver**

Create `runtime/src/runtime/wasmSession.ts`.

```ts
import { FixedStepLoop } from './fixedStepLoop';
import type { WasmRuntimeBridge, WasmRuntimeFrame, WasmRuntimeInputState } from './wasmBridge';

const DEFAULT_INPUT: WasmRuntimeInputState = {
  left: false,
  right: false,
  jump: false,
  jumpPressed: false,
  jumpReleased: false,
  restart: false,
};

export class WasmRuntimeSession {
  private readonly loop: FixedStepLoop;
  private input: WasmRuntimeInputState = { ...DEFAULT_INPUT };

  constructor(private readonly bridge: WasmRuntimeBridge) {
    this.loop = new FixedStepLoop({
      onStep: () => {
        throw new Error('Use stepOnce() for async wasm stepping');
      },
    });
  }

  setInputState(snapshot: Pick<WasmRuntimeInputState, 'left' | 'right' | 'jump' | 'restart'>): void {
    const nextJumpPressed = snapshot.jump && !this.input.jump;
    const nextJumpReleased = !snapshot.jump && this.input.jump;
    this.input = {
      left: snapshot.left,
      right: snapshot.right,
      jump: snapshot.jump,
      jumpPressed: nextJumpPressed,
      jumpReleased: nextJumpReleased,
      restart: snapshot.restart,
    };
  }

  async stepOnce(): Promise<WasmRuntimeFrame> {
    await this.bridge.setInput(this.input);
    await this.bridge.tick(1);
    const frame = await this.bridge.frame();
    this.input.jumpPressed = false;
    this.input.jumpReleased = false;
    return frame;
  }
}
```

The session stays intentionally small. Do not overbuild a second runtime abstraction yet.

- [ ] **Step 4: Implement the canvas frame renderer**

Create `runtime/src/render/wasmFrameRenderer.ts`.

```ts
import type { ResourceIndex } from '../types';
import type { WasmRuntimeFrame } from '../runtime/wasmBridge';
import { ResourceCache, makeBackgroundPathMap, makeSpriteFrameMap } from './resourceCache';

function rgbaToCss([r, g, b, a]: [number, number, number, number]): string {
  return `rgba(${r}, ${g}, ${b}, ${a / 255})`;
}

export async function renderWasmFrame(
  canvas: HTMLCanvasElement,
  frame: WasmRuntimeFrame,
  resources: ResourceIndex,
  basePath: string,
  cache: ResourceCache = new ResourceCache()
): Promise<void> {
  canvas.width = frame.width;
  canvas.height = frame.height;
  const context = canvas.getContext('2d');
  if (!context) {
    throw new Error('Canvas 2d context unavailable');
  }

  const backgroundPaths = makeBackgroundPathMap(basePath, resources);
  const spritePaths = makeSpriteFrameMap(basePath, resources);

  for (const command of frame.commands) {
    switch (command.kind) {
      case 'clear':
        context.clearRect(0, 0, frame.width, frame.height);
        context.fillStyle = rgbaToCss(command.colour);
        context.fillRect(0, 0, frame.width, frame.height);
        break;
      case 'drawBackground': {
        const path = backgroundPaths.get(command.backgroundId);
        if (!path) {
          continue;
        }
        const image = await cache.getImage(path);
        context.drawImage(image, command.x, command.y);
        break;
      }
      case 'drawSprite': {
        const sprite = spritePaths.get(command.spriteId);
        if (!sprite) {
          continue;
        }
        const image = await cache.getImage(sprite.imagePath);
        context.save();
        context.translate(command.x, command.y);
        if (command.angleDegrees !== 0) {
          context.rotate((command.angleDegrees * Math.PI) / 180);
        }
        if (command.xscale !== 1 || command.yscale !== 1) {
          context.scale(command.xscale, command.yscale);
        }
        context.drawImage(image, -command.originX, -command.originY);
        context.restore();
        break;
      }
      case 'fillRect':
        context.fillStyle = rgbaToCss(command.colour);
        context.fillRect(command.x, command.y, command.width, command.height);
        break;
      case 'present':
        break;
    }
  }
}
```

- [ ] **Step 5: Run the targeted frontend tests**

Run:

```powershell
npm --prefix runtime test -- wasmSession.test.ts
npm --prefix runtime test -- wasmFrameRenderer.test.ts
```

Expected:

```text
Test Files  2 passed
```

- [ ] **Step 6: Commit**

```powershell
git add runtime/src/runtime/wasmSession.ts runtime/src/runtime/wasmSession.test.ts runtime/src/render/wasmFrameRenderer.ts runtime/src/render/wasmFrameRenderer.test.ts
git commit -m "feat: add browser wasm session and frame renderer"
```

### Task 4: Wire Keyboard Input And WASM Rendering Into The Runtime Shell

**Files:**
- Modify: `runtime/src/ui/shell.ts`
- Modify: `runtime/src/main.test.ts`

- [ ] **Step 1: Write the failing shell test for keyboard-driven WASM stepping**

Extend `runtime/src/main.test.ts` with a browser-shell test that loads a wasm bridge, dispatches a left/jump key sequence, and expects `setInput`, `tick`, and `frame` to be called.

```ts
it('feeds keyboard input into the wasm runtime session and draws returned frames', async () => {
  const loadPackage = vi.fn(async () => samplePackage);
  const renderStaticRoom = vi.fn(async () => undefined);
  const wasmBridge = {
    backend: 'opengmk-wasm' as const,
    boot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    snapshot: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    frame: vi.fn(async () => ({ tick: 1, roomId: 0, width: 320, height: 240, commands: [{ kind: 'present' as const }] })),
    setInput: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    tick: vi.fn(async () => ({ tick: 1, roomId: 0, diagnostics: [] })),
    reset: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    selectRoom: vi.fn(async () => ({ tick: 0, roomId: 0, diagnostics: [] })),
    diagnostics: vi.fn(async () => []),
  };

  createRuntimeShell(root as unknown as HTMLElement, {
    loadPackage,
    renderStaticRoom,
    loadWasmBridge: async () => wasmBridge,
  });

  loadButton.click();
  await flushAsyncWork();

  doc.dispatchKeyboardEvent('keydown', 'ArrowLeft');
  pauseButton.click();
  await flushAsyncWork();

  expect(wasmBridge.setInput).toHaveBeenCalled();
  expect(wasmBridge.tick).toHaveBeenCalledWith(1);
  expect(wasmBridge.frame).toHaveBeenCalled();
});
```

- [ ] **Step 2: Run the shell test to verify it fails**

Run:

```powershell
npm --prefix runtime test -- main.test.ts
```

Expected:

```text
Property 'frame' does not exist on type 'WasmRuntimeBridge'
```

- [ ] **Step 3: Wire the shell to the new WASM session and keyboard events**

In `runtime/src/ui/shell.ts`, keep the existing TS fallback path, but add a WASM-specific session path.

```ts
import { renderWasmFrame } from '../render/wasmFrameRenderer';
import { WasmRuntimeSession } from '../runtime/wasmSession';
```

Track a lightweight keyboard state:

```ts
let wasmSession: WasmRuntimeSession | null = null;
const keyboardState = {
  left: false,
  right: false,
  jump: false,
  restart: false,
};
```

When the package boots through WASM:

```ts
if (wasmBridge) {
  const snapshot = await wasmBridge.boot(pkg);
  wasmSession = new WasmRuntimeSession(wasmBridge);
  activeBackend = 'wasm';
  const frame = await wasmBridge.frame();
  await renderWasmFrame(canvas, frame, pkg.resources, input.value);
  renderWasmDiagnostics(doc, diagnostics, await wasmBridge.diagnostics());
  status.textContent = `WASM runtime active: ${snapshot.roomName ?? 'room'} @ tick ${snapshot.tick}`;
}
```

Update the pause/reset/select handlers so the WASM path uses `wasmSession.stepOnce()`, `wasmBridge.reset()`, and `wasmBridge.selectRoom()` followed by `wasmBridge.frame()`.

```ts
pauseButton.addEventListener('click', async () => {
  if (!loadedPackage) {
    return;
  }
  if (activeBackend === 'wasm' && wasmBridge && wasmSession) {
    wasmSession.setInputState(keyboardState);
    const frame = await wasmSession.stepOnce();
    await renderWasmFrame(canvas, frame, loadedPackage.resources, input.value);
    renderWasmDiagnostics(doc, diagnostics, await wasmBridge.diagnostics());
    status.textContent = `WASM runtime active: ${frame.roomId ?? 'room'} @ tick ${frame.tick}`;
    return;
  }
  // existing TS path unchanged
});
```

Add document-level keyboard listeners scoped to the shell lifecycle:

```ts
const keyToAction = (key: string): keyof typeof keyboardState | null => {
  switch (key) {
    case 'ArrowLeft':
    case 'a':
    case 'A':
      return 'left';
    case 'ArrowRight':
    case 'd':
    case 'D':
      return 'right';
    case ' ':
    case 'Spacebar':
    case 'w':
    case 'W':
    case 'ArrowUp':
      return 'jump';
    case 'r':
    case 'R':
      return 'restart';
    default:
      return null;
  }
};

doc.addEventListener('keydown', (event) => {
  const action = keyToAction((event as KeyboardEvent).key);
  if (action) {
    keyboardState[action] = true;
  }
});

doc.addEventListener('keyup', (event) => {
  const action = keyToAction((event as KeyboardEvent).key);
  if (action) {
    keyboardState[action] = false;
  }
});
```

For the fake DOM in `runtime/src/main.test.ts`, add only the minimal keyboard-event helper needed by the test:

```ts
dispatchKeyboardEvent(type: string, key: string): void {
  for (const listener of this.listeners.get(type) ?? []) {
    listener({ key } as unknown as KeyboardEvent);
  }
}
```

- [ ] **Step 4: Run the runtime frontend suite**

Run:

```powershell
npm --prefix runtime test
npm --prefix runtime run build
```

Expected:

```text
Test Files  passed
vite build complete
```

- [ ] **Step 5: Commit**

```powershell
git add runtime/src/ui/shell.ts runtime/src/main.test.ts
git commit -m "feat: wire wasm runtime input and frame loop into shell"
```

### Task 5: Align Runtime Docs And Record The Browser Smoke Path

**Files:**
- Modify: `README.md`
- Modify: `docs/notes/package-format-v1-runtime.md`
- Modify: `docs/notes/runtime-gold-sample.md`

- [ ] **Step 1: Update the user-facing runtime instructions**

In `README.md`, replace the current “boot/tick/diagnostics only” WASM wording with the browser-loop wording.

```md
The current `iwm-runtime-web` bridge can now:

- boot a normalized runtime package
- accept keyboard input snapshots from the frontend shell
- advance deterministic ticks
- expose browser-consumable draw commands for the current room frame
- render that frame through the existing `runtime/` canvas shell
- return structured diagnostics and snapshots
```

Document the practical browser loop:

```powershell
git submodule update --init --recursive
cd runtime
npm install
cd ..
cargo test
cd runtime
npm test
npm run build
cd ..
$env:PATH='C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\Llvm\bin;' + $env:PATH
$env:CC='clang'
$env:CXX='clang++'
cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
cd runtime
npm run sync:wasm
npm run dev -- --host 127.0.0.1
```

And tell the user what to do in the shell:

- load `/packages/sample`
- click `Load Package`
- use `ArrowLeft` / `ArrowRight` / `Space` / `R`
- use `Pause` as the current single-step button for WASM bring-up

- [ ] **Step 2: Update the package and gold-sample notes**

In `docs/notes/package-format-v1-runtime.md`, add a new “Current browser host status” subsection:

```md
### Current Browser Host Status

The WASM-first runtime path now consumes this package in three layers:

- frontend package loader aggregates the JSON package
- `iwm-runtime-web` boots and ticks against that normalized payload
- the browser shell translates returned frame commands into real canvas draws using `resources/index.json`

This means `resources/` paths are now exercised by both the static-room viewer and the WASM bridge render path.
```

In `docs/notes/runtime-gold-sample.md`, add a concrete smoke checklist:

```md
## Current Browser Smoke Checklist

- [ ] Build the normalized package into `runtime/public/packages/sample/`
- [ ] Build `iwm_runtime_web.wasm`
- [ ] Run `npm --prefix runtime run sync:wasm`
- [ ] Start the Vite shell and load `/packages/sample`
- [ ] Verify the first room frame draws on canvas
- [ ] Verify left/right/jump/restart input reaches the WASM path
- [ ] Record the first visible blocker if player motion still lacks runner fidelity
```

- [ ] **Step 3: Run a final verification pass**

Run:

```powershell
cargo test
npm --prefix runtime test
npm --prefix runtime run build
$env:PATH='C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\Llvm\bin;' + $env:PATH
$env:CC='clang'
$env:CXX='clang++'
cargo build -p iwm-runtime-web --target wasm32-unknown-unknown
npm --prefix runtime run sync:wasm
```

Expected:

```text
test result: ok
vitest passed
vite build complete
Finished dev [unoptimized + debuginfo] target(s)
synced iwm_runtime_web.wasm into runtime/public/wasm/
```

- [ ] **Step 4: Commit**

```powershell
git add README.md docs/notes/package-format-v1-runtime.md docs/notes/runtime-gold-sample.md
git commit -m "docs: align wasm browser runtime usage"
```

## Recommended Execution Notes

- Implementation note (2026-05-21): the later runtime cleanup removed the TS gameplay fallback path. The shell now falls back to a static room viewer when the WASM bridge is unavailable.
- Execute this plan in order. Task 1 and Task 2 define the stable bridge contract; Task 3 and Task 4 should not be started before those interfaces settle.
- Do not widen scope into audio, mouse, externals, or deeper OpenGMK extraction while this slice is in progress.
- Keep the TS runtime path intact as fallback and comparison harness. This plan upgrades the WASM path to first-class browser bring-up, but it does not remove the fallback.
- If the first browser frame renders but player motion is still semantically weak, stop and record the exact blocker in `docs/notes/runtime-gold-sample.md` rather than starting a broad refactor.
