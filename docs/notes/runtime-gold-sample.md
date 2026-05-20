# Runtime Gold Sample

This document captures the current validation samples for runtime bring-up.

These samples remain useful under the WASM-first direction as runtime-core bring-up and fidelity validation targets.

Important local-path note:

- these sample paths are local development paths, not tracked sample binaries
- a fresh clone may not contain `samples/local/iwanna-examples/` until you add local test data

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

## Expected Validation Behaviors

Under the WASM-first runtime direction, Phase 4 validation for `IWBT_Dife` should eventually prove:

- [ ] Package loads successfully
- [ ] Extracted runtime core boots into the manifest default room or equivalent first-room path
- [ ] Browser host can drive deterministic ticks
- [ ] Player movement and collision fidelity can be validated against runner semantics
- [ ] At least one room transition can be exercised through the WASM-hosted runtime path
- [ ] Diagnostics remain explicit when unsupported logic, externals, or host gaps are encountered

## Out-of-Scope for Phase 4

- Full beatability (not the goal)
- Broad GM8 compatibility
- Advanced trap timing parity
- Particle systems
- Save/load functionality
- Menu systems
- Complex DLL dependencies

## Known Risky Mechanics or Rooms

No curated risky-room list is maintained yet.

When a specific room or mechanic blocks package build, boot, tick, input, collision, or room transition bring-up, record it here with the exact sample path and blocker summary.

## Implementation Guidance

The gold sample drives implementation priorities:

- If `IWBT_Dife` exposes a host-bound blocker in OpenGMK runtime extraction, solve that before improving the old TS gameplay runtime
- If parser output lacks data needed for the WASM-hosted path, extend parser outputs deliberately
- Do not optimize for multiple games before one sample can boot and tick correctly through the target runtime architecture

## References

- WASM-first runtime plan: `docs/superpowers/plans/2026-05-20-opengmk-wasm-first-runtime.md`
- Package format: `docs/notes/package-format-v1-runtime.md`
- Design spec: `docs/superpowers/specs/2026-05-19-iwanna-gm8-web-engine-design.md`

Historical note:

- `docs/superpowers/plans/2026-05-20-minimal-playable-runtime.md` is retained only as the previous TS-first plan and should not drive new gameplay-engine work
