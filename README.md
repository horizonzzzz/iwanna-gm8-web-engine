# iwanna-gm8-web-engine

[中文说明](README-CN.md)

Browser-playable IWanna engine for a curated legacy GM8 compatibility subset.
The current release line is `0.2.0-beta.1`.

> [!IMPORTANT]
> Beta means the upload-to-Canvas product path is available. It does not mean
> arbitrary GM8 games are fully compatible or finishable.

## Beta Product

The deployed application has two browser surfaces:

- `/` — minimal user flow: upload one `.exe` or `.zip`, parse it, and play on Canvas
- `/shell` — retained developer shell for package inspection and runtime diagnostics

The single-container request path is:

```text
upload -> iwm-api -> detector -> parser -> runtime-v1 validator
       -> /data/games/<sha256> -> WASM runtime -> Canvas
```

The Rust service never executes uploaded EXE or DLL files. It parses supported
GM8 data, validates the generated package, and only then publishes browser-safe
package files.

## Run The Beta

Initialize the parser dependency and build the image:

```powershell
git submodule update --init --recursive
.\scripts\build-beta.ps1
docker run --rm --name iwm-beta -p 3000:3000 -v iwm-data:/data iwm-beta:0.2.0-beta.1
```

Open `http://127.0.0.1:3000`. The health endpoint is
`http://127.0.0.1:3000/healthz`.

The volume contains generated runtime packages only. Uploaded originals stay in
unserved staging directories and are removed after each request. Beta packages
expire after 24 hours; at most 16 generated packages are retained.

The build script pulls the three base images sequentially before invoking
`docker build`, which is more reliable on networks where BuildKit's concurrent
registry authorization fails.

## Local Development

Prerequisites:

```powershell
git submodule update --init --recursive
npm --prefix runtime install
rustup target add wasm32-unknown-unknown
```

Build and sync the release WASM module, then run API and Vite in separate
terminals:

```powershell
cargo build -p iwm-runtime-web --release --target wasm32-unknown-unknown
npm --prefix runtime run sync:wasm
npm --prefix runtime run build
cargo run -p iwm-api
```

```powershell
npm --prefix runtime run dev -- --host 127.0.0.1
```

Vite serves `http://127.0.0.1:4173` and proxies `/api` and `/games` to the Rust
service on port `3000`.

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `POST` | `/api/v1/games` | Multipart upload in the `game` field |
| `GET` | `/games/{sha256}/*` | Validated generated package assets |
| `GET` | `/healthz` | Process health and version |

Successful upload response:

```json
{
  "id": "sha256",
  "status": "ready",
  "compatibility": "partial",
  "package_url": "/games/sha256",
  "warnings": []
}
```

Current trust-boundary limits are 512 MiB per upload, 4,096 ZIP entries,
512 MiB per expanded ZIP entry, 1 GiB total expansion/generated package size,
one concurrent parser, and a 120-second HTTP parse window. ZIP traversal and
special-file entries are rejected.

## Verify

```powershell
cargo test
cargo clippy -p iwm-api -p iwm-detector -p iwm-parser --all-targets --locked --no-deps -- -D warnings
npm --prefix runtime test
npm --prefix runtime run build
npm --prefix runtime run test:browser
docker build -t iwm-beta:0.2.0-beta.1 .
```

Browser/sample tests require fresh release WASM and a freshly generated local
package. Never treat stale files under `runtime/public/` as release evidence.

## Developer CLI And Samples

The CLI remains the compatibility investigation tool:

```powershell
cargo run -p iwm-cli -- sample-audit --input "C:\path\to\game" --package-output .\runtime\public\packages\candidate --report-output .\target\sample-audits\candidate.json --ticks 600
cargo run -p iwm-cli -- runtime-scenario --input .\runtime\public\packages\candidate --scenario .\docs\notes\runtime-scenarios\candidate-title-idle.json --ticks 600
```

Local copyrighted samples belong under `samples/local/iwanna-examples/` and
must not be committed. `IWBT_Dife` remains the L1 regression target;
ArioTrials is the current L2 development sample.

## Repository Layout

- `crates/iwm-api/` — upload API, generated-package serving, and static web host
- `crates/iwm-detector/` — package inventory, safe ZIP expansion, and engine verdict
- `crates/iwm-parser/` — GM8 parsing, resource export, and logic lowering
- `crates/iwm-runtime-model/` — package schema and validator
- `crates/iwm-runtime-host/` — project-owned host boundaries
- `crates/iwm-runtime-core/` — deterministic runtime behavior
- `crates/iwm-runtime-web/` — browser/WASM bridge
- `crates/iwm-cli/` — developer diagnostics and sample workflows
- `runtime/` — public user web, retained `/shell`, input/audio/render glue
- `docs/` — current notes plus clearly marked historical design records
- `vendor/` — pinned upstream references and parser dependency

## Compatibility Boundary

The current runtime covers a meaningful IWanna-critical subset, including the
proven movement, collision, lifecycle, savepoint, room, audio, and drawing paths
recorded in `docs/notes/runtime-wasm-gap-analysis.md`. Remaining gaps include
broader GML/GM8 parity, mouse semantics, advanced Draw behavior, per-frame masks,
multi-view cameras, broad file/audio behavior, and DLL/external calls.

The API reports compatibility as `supported`, `partial`, or `blocked`; it does
not promise universal playability.

## Current Documentation

- `docs/notes/beta-release.md`
- `docs/notes/package-format-v1-runtime.md`
- `docs/notes/runtime-wasm-gap-analysis.md`
- `docs/notes/runtime-performance-optimization.md`
- `docs/notes/runtime-gold-sample.md`
- `docs/notes/runtime-vendor-reference-map.md`
- `docs/notes/opengmk-host-coupling-audit.md`
- `docs/notes/testing-strategy.md`

The original MVP design under `docs/superpowers/specs/` is retained as a
historical baseline, not as the current deployment specification.

## Licensing

Project-owned source is MIT licensed. The parser and API binary link OpenGMK
`gm8exe`, which declares `GPL-2.0-only`; distributing the API binary or Docker
image therefore requires the corresponding GPL compliance described in
`NOTICE.md`. See `vendor/README.md` before publishing artifacts.
