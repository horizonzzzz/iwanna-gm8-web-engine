# Local Samples

This directory stores development sample data used to validate detector, parser, and later runtime behavior.

## Layout

- `local/iwanna-examples/`

Important repository note:

- this repository does not commit the actual sample binaries
- a fresh clone may contain only this README until you add local sample data under `samples/local/`
- scripts and plans may refer to these paths as expected local development locations, not guaranteed tracked files

Current local categories under `local/iwanna-examples/`:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

## Purpose

These samples are used for:

- engine detection validation
- parser smoke testing
- compatibility triage
- future runtime regression checks

## Current Usage

These samples are used for:

- detector validation
- parser/package smoke testing
- runtime package generation
- runtime regression checks

Important:

- the local sample directories may be absent in a fresh clone
- generated runtime packages under `runtime/public/packages/` are not a substitute for the raw local sample corpus

## Important Notes

- These files are local development assets, not canonical project source
- Do not treat current category labels as permanent truth
- Reclassify samples when stronger parser evidence becomes available
- Be careful with redistribution and copyright
