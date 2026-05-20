# Package Format V1 Runtime

Current emitted runtime package directory contents:

- `manifest.json`
- `rooms.json`
- `objects.json`
- `scripts.ir.json`
- `analysis.json`
- `resources/index.json`
- `resources/sprites/...`
- `resources/backgrounds/...`
- `resources/audio/...`

This package is runtime-consumable but still phase-limited.

Included in this phase:

- browser-ready sprite exports
- browser-ready background exports
- audio file exports
- normalized room instance placements
- normalized object event table
- first logic envelope in `scripts.ir.json`

Still deferred:

- fixed-step gameplay execution
- collision runtime
- player control
- death and respawn
- room-transition simulation
