# iwanna-gm8-web-engine

[中文说明](README-CN.md)

Browser-playable IWanna MVP targeting legacy GM8-style fangames.

## Current Direction

The active pipeline is:

1. detect whether an original game package is likely a supported GM8-style target
2. parse targetable packages with the backend/tooling crates
3. normalize them into a project-owned runtime package
4. execute that package through the browser-facing WASM runtime path

Phase 4 is in progress. The browser shell remains a package loader, inspector,
diagnostics surface, and host bridge. Runtime fidelity work is now centered on a
WASM-first path with OpenGMK-informed semantics behind project-owned boundaries;
the removed TypeScript gameplay runtime is not the long-term engine direction.

Current implemented pieces:

- `crates/iwm-detector/` detects likely target packages
- `crates/iwm-parser/` reads GM8 assets and builds normalized runtime packages
- `crates/iwm-cli/` exposes detection, package building, validation, and runtime diagnostics
- `crates/iwm-runtime-model/` owns shared package schemas and validation
- `crates/iwm-runtime-host/` defines host-boundary traits and default/headless helpers
- `crates/iwm-runtime-core/` runs deterministic runtime-core behavior and the current lowered-logic slice
- `crates/iwm-runtime-web/` exposes the browser-loadable WASM bridge
- `runtime/` loads packages, drives the WASM bridge, forwards input, renders frame commands, and shows diagnostics

The runtime package currently includes raw preserved logic, structured lowered
logic, browser-ready resources, sprite collision bounds/masks, and GM font atlas
metadata. The lowered runtime path covers an IWanna-critical subset, but this is
not a full GM8 runner.

## Current Docs

Use these as the current source of truth:

- `README.md`
- `AGENTS.md`
- `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-performance-optimization.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `docs/notes/testing-strategy.md`

Older design specs under `docs/superpowers/specs/` may be useful historical
context, but current notes and code take precedence when they disagree.

## Setup

```powershell
git submodule update --init --recursive
npm --prefix runtime install
rustup target add wasm32-unknown-unknown
```

On Windows, build the WASM target from a Visual Studio Developer Command Prompt
or otherwise ensure `clang` and `clang++` are on `PATH`.

## Verify

```powershell
cargo test
npm --prefix runtime test
npm --prefix runtime run test:browser
npm --prefix runtime run build
```

The default Rust suite uses in-memory fixtures. Local sample-backed runtime-core
tests are behind an explicit feature so stale generated sample packages do not
break ordinary verification:

```powershell
cargo test -p iwm-runtime-core --features local-sample-tests
```

Run that feature only after rebuilding and validating
`runtime/public/packages/sample` from your local corpus.

`npm --prefix runtime run test:browser` expects the local browser smoke
prerequisites used by the runtime shell.

## Build The WASM Bridge

```powershell
$env:PATH='C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\Llvm\bin;' + $env:PATH
$env:CC='clang'
$env:CXX='clang++'
cargo build -p iwm-runtime-web --release --target wasm32-unknown-unknown
npm --prefix runtime run sync:wasm
```

This copies `target\wasm32-unknown-unknown\release\iwm_runtime_web.wasm` to
`runtime\public\wasm\iwm_runtime_web.wasm`.

## Build A Runtime Package

```powershell
cargo run -p iwm-cli -- detect --input C:\path\to\game
cargo run -p iwm-cli -- build-package --input C:\path\to\game --output .\runtime\public\packages\sample
cargo run -p iwm-cli -- validate-package --input .\runtime\public\packages\sample
```

The shell default package path is `/packages/sample`, which maps to
`runtime\public\packages\sample\`.

`validate-package` checks the normalized runtime package contract before browser
smoke, including manifest counts, sparse id references, resource references, and
logic block presence across `scripts.ir.json`, `logic.raw.json`, and
`logic.lowered.json`.

## Run Runtime Diagnostics

After a package validates, use CLI diagnostics to run the headless runtime and
rank lowered-runtime blockers before adding new GM helpers:

```powershell
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --ticks 600
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --select-room 143 --ticks 240 --press-keys 16
cargo run -p iwm-cli -- runtime-diagnostics --input .\runtime\public\packages\sample --input-script .\runtime-input-script.json --trace-player --trace-every 1
```

Useful options:

- `--select-room <room_id>` enters a room before ticking
- `--preselect-ticks <n>` advances the boot room before manual room selection
- `--ticks <n>` controls the diagnostic window
- `--press-keys`, `--hold-keys`, and `--input-script` drive virtual-key input
- `--trace-player` adds compact player behavior traces
- `--trace-output <path>` writes the full diagnostics JSON to a file

Diagnostics JSON includes grouped runtime blockers, runtime lifecycle events,
and optional player trace summaries. See `docs/notes/runtime-scenarios/` for
checked-in input-script examples.

## Run The Browser Shell

```powershell
npm --prefix runtime run dev -- --host 127.0.0.1
```

Then open `http://127.0.0.1:4173`.

Current shell behavior:

- loads normalized packages from a package path input
- boots the WASM bridge when available
- falls back to static package inspection when WASM is missing or boot fails
- forwards raw GM virtual-key hold/press/release state to the runtime host
- auto-runs the WASM path at the active room speed, with Pause/Resume controls
- renders the runtime frame on canvas
- exposes HUD telemetry and a copy-first plain-text runtime report
- keeps package inspectors as secondary read-only tabs

Current hand-test controls:

- `ArrowLeft` / `A` for left
- `ArrowRight` / `D` for right
- `Space` / `ArrowUp` / `W` for jump input
- `R` as raw package keyboard input
- `Reset` button for an explicit shell reset

Local-only generated paths:

- `runtime/public/packages/` is intentionally empty in git except for `.gitkeep`
- `runtime/public/wasm/iwm_runtime_web.wasm` is generated locally and not committed
- `samples/local/iwanna-examples/` may be absent in a fresh clone

## Repository Layout

- `docs/` - project documentation, notes, and design specs
- `crates/iwm-detector/` - target detection and package inventory logic
- `crates/iwm-parser/` - GM8 parsing, package building, resource export, and logic lowering
- `crates/iwm-cli/` - developer CLI
- `crates/iwm-runtime-model/` - shared runtime package schema and validation
- `crates/iwm-runtime-host/` - runtime host-boundary types and helpers
- `crates/iwm-runtime-core/` - deterministic runtime-core behavior
- `crates/iwm-runtime-web/` - WASM/browser bridge surface
- `runtime/` - browser shell, diagnostics UI, package loading, and rendering glue
- `samples/local/iwanna-examples/` - local sample corpus when populated
- `vendor/` - upstream reference submodules

Planned later area:

- `backend/`

## Samples

The local sample corpus is organized under `samples/local/iwanna-examples/` when
populated locally.

Current categories:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Treat labels as working development categories, not final truth. Do not commit
copyrighted sample binaries.

## Vendored References

Tracked references:

- `vendor/OpenGMK/`
- `vendor/GM8Decompiler/`

Use them for GM8 executable handling, parser assumptions, and runtime semantic
study. Some OpenGMK ecosystem components may be `GPL-2.0-only`; any direct
dependency or code reuse must be a deliberate licensing decision.

## Scope Rules

- focus on mainstream legacy GM8-style IWanna fangames
- keep parser/runtime contracts stable unless gold-sample evidence requires targeted expansion
- keep browser work focused on WASM host integration, diagnostics, controls, and rendering
- do not re-expand a parallel TypeScript gameplay runtime
- do not claim full GM8 parity from the current IWanna-critical subset
