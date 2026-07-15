# Beta Release 0.2

> **Status:** Active deployment and product boundary for `0.2.0-beta.1`.

This note supersedes the early MVP deployment assumptions. Runtime fidelity and
package details remain owned by their dedicated current notes.

## Release Shape

The first Beta deliberately has four product pieces:

1. a minimal user web at `/`
2. a Rust service that accepts and parses IWanna packages
3. one Docker image containing API, release WASM, and built frontend
4. the existing developer Web Shell retained at `/shell`

There is one gameplay implementation. Both browser surfaces use the same
`iwm-runtime-web` WASM bridge, session, keyboard, audio, package loader, and
Canvas renderer.

## Request Flow

```text
POST multipart .exe/.zip
  -> private staging directory
  -> iwm-detector
  -> iwm-parser
  -> iwm-runtime-model validation
  -> atomic publish at /data/games/<sha256>
  -> browser loads /games/<sha256>
  -> WASM runtime
  -> Canvas
```

The API serves the frontend, generated package assets, and health endpoint from
the same origin. No CORS layer, reverse proxy, database, queue, account system,
or object store is part of this release.

## Endpoints

- `POST /api/v1/games` — multipart field `game`
- `GET /games/{sha256}/*` — generated package JSON/assets only
- `GET /healthz` — process version and health

Upload responses use `ready`, while errors use `failed`. Compatibility remains
separate and uses the package contract values `supported`, `partial`, or
`blocked`.

## Trust Boundary

The service does not execute uploaded EXE or DLL files and never serves the
original upload directory.

Current fixed Beta limits:

- upload: 512 MiB
- ZIP entries: 4,096
- one ZIP entry: 512 MiB expanded
- total ZIP expansion: 1 GiB
- generated package: 1 GiB
- parser concurrency: one
- HTTP parse window: 120 seconds
- retained generated packages: 16
- generated package TTL: 24 hours

ZIP extraction rejects parent traversal, absolute/drive paths, symlinks, and
other special files. Generated packages are validated before an atomic move
into the served directory. A timed-out `spawn_blocking` parse cannot be safely
cancelled; it retains the only parse permit until the parser actually returns.
Move this work to an isolated job process only when measured traffic requires
cancellation or stronger resource isolation.

## Browser Behavior

The public page contains only upload, progress/error state, Canvas, reset, and a
link to diagnostics. It fails closed when package loading, validation, WASM
loading, or runtime boot fails.

The diagnostic shell keeps its package-path input, room selection, telemetry,
inspectors, and static viewer fallback. Static fallback is a developer aid and
must not be presented as successful gameplay on `/`.

## Storage

`/data/staging` is private and request-scoped. `/data/games` contains only
validated runtime packages addressed by upload SHA-256. Re-uploading identical
content reuses a valid generated package.

This local-volume policy is intentionally small. Add persistent metadata,
object storage, or a job queue only after a real multi-instance deployment
needs them.

## Compatibility Claim

This is a curated compatibility Beta, not a general GM8 emulator release.
Two local samples prove meaningful paths, but unsupported GML, lifecycle,
rendering, input, camera, audio, file, collision-mask, and external behavior
remain documented in `runtime-wasm-gap-analysis.md`.

The upload API accepting a file proves only detect/parse/validate readiness.
Playable compatibility still requires runtime evidence.

## Docker And Licensing

`Dockerfile` builds the API, release WASM, and frontend, then runs only
`iwm-api` in the final image. `/data` is a volume and `/healthz` is the Docker
health check.

The API links OpenGMK `gm8exe` (`GPL-2.0-only`). Distributing the binary or
image requires matching corresponding source, build scripts, license text, and
the pinned submodule revision. `NOTICE.md` and OCI revision metadata make this
obligation explicit; they do not replace a distributor's compliance review.

The tag-triggered workflow at `.github/workflows/release.yml` validates a `v*`
tag against the Cargo workspace version, requires matching current-version
sections in `CHANGELOG.md` and `CHANGELOG.zh-CN.md`, publishes one Docker Hub
manifest for `linux/amd64` and `linux/arm64`, and creates or updates the matching
GitHub Release with those English and Chinese sections. It requires the
repository secrets `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN`.

## Release Gates

Before publishing a Beta image:

1. build fresh release WASM and frontend assets
2. run the workspace Rust suite and frontend suite
3. run browser smoke against fresh package artifacts
4. upload one local L1 sample through the API and confirm Canvas advances
5. build the Linux Docker image and verify `/healthz`, `/`, and `/shell`
6. verify the image revision has matching public corresponding source

Do not ship copyrighted sample binaries or generated packages in the image.
The Beta Clippy gate covers `iwm-api`, `iwm-detector`, and `iwm-parser` with
dependencies excluded. Strict full-workspace Clippy still exposes pre-existing
runtime-core lint debt, while linting the vendored `gm8exe` revision separately
reports its own warnings and unsafe-buffer lints. Neither is silently patched as
part of the service release; revisit them in their owning cleanup/upstream work.
