# Sample Corpus Notes

Current project-local sample root:

- `C:\Users\59164\work\playground\iwanna-gm8-web-engine\samples\local\iwanna-examples`

Current local categories:

- `gm8-core`
- `gm8-extended`
- `needs-manual-check`
- `non-target`

Detector development order:

1. Run all `gm8-core` samples and confirm `gm8-likely` or `unknown`
2. Run all `non-target` samples and confirm they are not classified as `gm8-likely`
3. Review `needs-manual-check` output and record missing heuristics
4. Defer DLL-heavy edge cases until detector stability is proven

Practical rule:

- future scripts, plans, and local smoke tests should prefer this project-local sample path
- do not assume the old desktop path exists anymore
