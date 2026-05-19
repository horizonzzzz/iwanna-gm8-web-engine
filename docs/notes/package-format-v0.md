# Package Format V0

Current emitted package directory contents:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.json`
- `analysis.json`

This phase emits structural summaries only.

Not yet included:

- sprite exports
- audio exports
- background exports
- script IR
- room instance normalization for runtime execution
- browser-ready resources directory

Purpose of V0:

- stabilize parser output shape
- let downstream runtime work start from JSON structure
- keep resource export work decoupled from parser integration
