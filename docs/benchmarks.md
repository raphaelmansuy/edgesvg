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
- robust benchmark score with tail-aware percentiles
- deterministic balanced sampling for limited golden runs
- per-group summaries
- dataset mix by group and analyzed type/complexity
- explicit quality gate failures
- per-entry metrics
- lowest-fidelity assets for triage
- JSON for tooling
- Markdown for review

Generated locations:

- `benchmark_runs/golden_smoke/artifacts`
- `benchmark_runs/golden_smoke/reports`
- `benchmark_runs/golden_sample/artifacts`
- `benchmark_runs/golden_sample/reports`
- `benchmark_runs/golden_full/artifacts`
- `benchmark_runs/golden_full/reports`
- `benchmark_runs/optimization_frontier/trials`
- `benchmark_runs/optimization_frontier/reports`

## Golden Data Notes

- `golden_data/icons` now uses `Heroicons` optimized `24/solid` SVGs as the icon reference corpus.
- Golden SVG references are rasterized at a minimum longest-side resolution of `1024px` during `benchmark-golden` runs to preserve enough signal for edge and topology metrics.
- `benchmark-golden` now clears stale artifacts inside `artifacts/rendered_inputs` and `artifacts/vectorized` before each run so outdated PNGs and SVGs cannot pollute new reports.
- When a `limit` is provided, the benchmark uses deterministic balanced sampling across top-level golden groups instead of truncating the lexically first files.
- The previous legacy icon corpus is backed up under `benchmark_runs/dataset_backups/icons_legacy_feather`.

## Library Access

The same report is available through Rust and the SDK contract:

- Rust: `benchmark_directory`, `benchmark_golden_data`
- SDK layer: `benchmark`, `benchmark_golden`
