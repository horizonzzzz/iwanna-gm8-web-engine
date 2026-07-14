# Vendored References

This directory stores external upstream repositories used for research, format study, and narrow integration.

These repositories are intentionally kept separate from the main project code.

## Submodule Policy

The heavyweight upstream checkouts under this directory are tracked as git submodules.

Repository rule:

- keep `vendor/README.md` tracked
- keep `vendor/OpenGMK/` and `vendor/GM8Decompiler/` attached as git submodules
- initialize them after clone with `git submodule update --init --recursive`

Practical implication:

- documentation may refer to `vendor/OpenGMK/` or `vendor/GM8Decompiler/` as available repository paths
- a fresh clone of this repository should fetch submodules before parser work
- path dependencies on vendored code now resolve through tracked submodule locations

## Current Repository Reality

The project now uses vendored references for both parser and runtime guidance:

- `gm8exe` remains the intended narrow parser dependency boundary
- `gm8emulator` is the primary runtime-semantics reference for the WASM-first runtime path

When runtime-semantics assumptions change, update the relevant runtime notes alongside code changes.

## Repositories

### `OpenGMK/`

Upstream:

- https://github.com/OpenGMK/OpenGMK

Primary reasons this repo is included:

- inspect `gm8exe` structures and parsing entrypoints
- inspect `gm8emulator` behavior when runner semantics need study
- inspect asset layout and GM8 format handling

Current use in this project:

- reference `gm8exe`
- path-depend on `vendor/OpenGMK/gm8exe` through the narrow parser adapter boundary

Do not:

- copy large pieces of code into the main project without a deliberate license decision
- tightly couple project architecture to the entire OpenGMK workspace

### `GM8Decompiler/`

Upstream:

- https://github.com/OpenGMK/GM8Decompiler

Primary reasons this repo is included:

- study GM8 executable-to-project recovery behavior
- compare file support and edge cases against `gm8exe`
- understand game-data extraction limitations

Expected near-term use in this project:

- reference implementation details
- validation when parsing odd GM8 executables

## License Warning

At least part of the OpenGMK ecosystem uses GPL-2.0-only licensing.

That means:

- using code directly is not just a technical decision
- shipping binaries that incorporate these dependencies may impose GPL obligations on this project

Current project rule:

- vendor these repos as tracked submodules for study and controlled integration experiments
- keep all project-owned logic behind narrow adapter boundaries
- treat API/Docker redistribution as GPL-sensitive and follow `NOTICE.md`

## Integration Boundary

If `gm8exe` is used directly, isolate it behind:

- `crates/iwm-parser/src/gm8_adapter.rs`

That file should be the only place that knows about the upstream parser API.

## Update Policy

When updating these submodules:

1. update the submodule under `vendor/`
2. re-check parser entrypoints and public structs
3. re-run local detector and package-builder smoke tests
4. document any API drift in `docs/notes/`
