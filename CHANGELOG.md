# Changelog

All notable changes to this project are documented here.

中文版本：[CHANGELOG.zh-CN.md](CHANGELOG.zh-CN.md)

## [Unreleased]

No changes yet.

## [0.2.0-beta.2] - 2026-07-15

### Changed

- Release automation now publishes the English and Chinese changelog sections in the GitHub Release.
- Machine-specific assistant skill and hook configuration is no longer tracked; local `.claude/` and `.codex/` directories are ignored.

### Fixed

- Normalize missing visible room backgrounds and tile references during package generation, preserving strict validation with explicit warnings.

## [0.2.0-beta.1] - 2026-07-14

### Added

- Browser upload-to-Canvas gameplay flow for supported GM8-style IWanna games.
- Rust detector, parser, package validator, upload API, generated-package serving, and health endpoint.
- WASM-first runtime bridge with the retained `/shell` diagnostics surface.
- CLI workflows for detection, package generation and validation, sample audits, and runtime diagnostics.
- Single-container Docker release path with multi-architecture image publishing for `linux/amd64` and `linux/arm64`.
- Runtime package v1 with normalized resources, room/object data, raw and lowered logic, compatibility analysis, and cross-file validation.

### Changed

- Uploads are validated before generated packages are published.
- The public `/` route fails closed on package, WASM, or runtime errors; static fallback remains available only in `/shell`.
- Compatibility is reported as `supported`, `partial`, or `blocked` instead of implying universal GM8 support.
- The runtime covers the current IWanna-critical movement, collision, lifecycle, room, audio, savepoint, and drawing slices while documenting remaining GM8 gaps.

### Known limitations

- This is a curated GM8 compatibility Beta, not a general GM8 emulator.
- Broader GML/GM8 semantics, mouse input, advanced drawing, multi-view cameras, complete file/audio behavior, and DLL/external calls remain unsupported or partial.
- Release artifacts incorporating OpenGMK `gm8exe` remain subject to its GPL-2.0-only compliance requirements.
