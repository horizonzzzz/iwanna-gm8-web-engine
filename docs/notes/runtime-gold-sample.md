# Runtime Gold Sample

This document captures the gold sample chosen for Phase 4 minimal playable runtime implementation.

## Primary Gold Sample

**Path:** `samples/local/iwanna-examples/gm8-core/IWBT_Dife`

## Why IWBT_Dife

1. **Already used in existing smoke tests** - the project has prior experience generating packages from this sample
2. **Meaningful `source-only` blocks** - contains enough source-only blocks to serve as a forcing function for the parser/runtime boundary
3. **Straightforward IWanna mechanics** - core movement, jumping, and hazard patterns
4. **Known room structure** - manageable number of rooms for initial implementation
5. **Room transitions present** - allows testing of basic room-goto functionality

## Fallback/Comparison Sample

**Path:** `samples/local/iwanna-examples/gm8-core/I Wanna Kill the Kamilia Ver. Final`

**Reasoning:**
- `Kamilia` currently produces mostly `action-list` blocks (useful contrast when debugging IR execution)
- Can serve as a secondary validation target after gold sample is playable

## Expected Milestone Behaviors

Phase 4 targets the following gameplay path for IWBT_Dife:

- [ ] Package loads successfully
- [ ] Runtime boots into the manifest default room
- [ ] Player can move left and right
- [ ] Player can jump
- [ ] Player collides with solid terrain
- [ ] Player dies on at least one basic hazard
- [ ] Player respawns correctly (room restart or spawn-point reset)
- [ ] At least one room transition works (triggered through supported action subset)
- [ ] Runtime diagnostics remain explicit when unsupported logic is encountered

## Out-of-Scope for Phase 4

- Full beatability (not the goal)
- Broad GM8 compatibility
- Advanced trap timing parity
- Particle systems
- Save/load functionality
- Menu systems
- Complex DLL dependencies

## Known Risky Mechanics or Rooms

TBD - will be updated after initial gold sample package analysis and runtime execution testing.

## Implementation Guidance

The gold sample drives implementation priorities:

- If `IWBT_Dife` needs action X to be playable, implement action X first
- If a specific `source-only` block blocks critical gameplay, lower that block or classify it as unsupported-but-noncritical
- Do not optimize for multiple games before one gold sample is playable

## References

- Implementation plan: `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md`
- Package format: `docs/notes/package-format-v1-runtime.md`
- Design spec: `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`