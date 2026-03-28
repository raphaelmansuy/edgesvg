# Benchmarks

## Commands

```bash
make bench-smoke
make bench-sample
make bench-full
```

## Modes

| Target | Purpose |
|---|---|
| `bench-smoke` | Fast sanity run on a small golden subset |
| `bench-sample` | Main regression loop with baseline comparison |
| `bench-full` | Full golden corpus verification |
| `optimize-frontier` | OODA-style parameter search |

## Outputs

Benchmark reports contain:

- overall averages
- per-group summaries
- per-entry metrics
- lowest-fidelity assets for triage
- JSON for tooling
- Markdown for review

Generated locations:

- `benchmark_runs/golden_smoke`
- `benchmark_runs/golden_sample`
- `benchmark_runs/golden_full`
- `benchmark_runs/optimization_frontier`

## Golden Data Notes

- `golden_data/icons` now uses `Heroicons` optimized `24/solid` SVGs as the icon reference corpus.
- Golden SVG references are rasterized at a minimum longest-side resolution of `1024px` during `benchmark-golden` runs to preserve enough signal for edge and topology metrics.
- `benchmark-golden` now clears stale `rendered_inputs/` and `vectorized/` artifacts inside the target work dir before each run so outdated low-resolution PNGs cannot pollute new reports.
- The previous legacy icon corpus is backed up under `benchmark_runs/dataset_backups/icons_legacy_feather`.

## Library Access

The same report is available through Rust and the SDK contract:

- Rust: `benchmark_directory`, `benchmark_golden_data`
- SDK layer: `benchmark`, `benchmark_golden`
