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

## Outputs

Benchmark reports contain:

- overall averages
- reliability score based on a weighted geometric mean of fidelity, edge, color, foreground, gradient, and topology terms
- local SSIM p10 to expose localized collapses that global SSIM can hide
- efficiency score based on byte and path budgets normalized by analyzed asset class
- robust benchmark score with tail-aware percentiles and balanced stratum weighting
- dataset health score based on group, subgroup, and complexity coverage plus distribution balance
- balanced group and type/complexity scores
- balanced subgroup score so one narrow source family cannot dominate a "good" result
- deterministic balanced sampling for limited golden runs
- per-group summaries
- dataset mix by group and analyzed type/complexity
- source-complexity mix for limited-set coverage review
- explicit quality gate failures
- per-entry metrics
- lowest-fidelity assets for triage
- lowest-efficiency assets for path and byte budget triage
- lowest local-SSIM, edge, color, and topology entries for metric-specific triage
- JSON for tooling
- Markdown for review

Generated locations:

- `benchmark_runs/golden_smoke/artifacts`
- `benchmark_runs/golden_smoke/reports`
- `benchmark_runs/golden_smoke/reports/summary`
- `benchmark_runs/golden_smoke/reports/groups`
- `benchmark_runs/golden_smoke/reports/entries`
- `benchmark_runs/golden_smoke/reports/entries/by_subgroup`
- `benchmark_runs/golden_smoke/reports/failures`
- `benchmark_runs/golden_smoke/reports/failures/by_metric`
- `benchmark_runs/golden_smoke/reports/dataset`
- `benchmark_runs/golden_sample/artifacts`
- `benchmark_runs/golden_sample/reports`
- `benchmark_runs/golden_sample/reports/summary`
- `benchmark_runs/golden_sample/reports/groups`
- `benchmark_runs/golden_sample/reports/entries`
- `benchmark_runs/golden_sample/reports/entries/by_subgroup`
- `benchmark_runs/golden_sample/reports/failures`
- `benchmark_runs/golden_sample/reports/failures/by_metric`
- `benchmark_runs/golden_sample/reports/dataset`
- `benchmark_runs/golden_full/artifacts`
- `benchmark_runs/golden_full/reports`
- `benchmark_runs/golden_full/reports/summary`
- `benchmark_runs/golden_full/reports/groups`
- `benchmark_runs/golden_full/reports/entries`
- `benchmark_runs/golden_full/reports/entries/by_subgroup`
- `benchmark_runs/golden_full/reports/failures`
- `benchmark_runs/golden_full/reports/failures/by_metric`
- `benchmark_runs/golden_full/reports/dataset`
## Golden Data Notes

- `golden_data/icons` now uses `Heroicons` optimized `24/solid` SVGs as the icon reference corpus.
- `golden_data/diagrams/w3c` now includes a broader slice of W3C SVG Authoring examples so diagram, layout, text, symbol reuse, and responsive-viewbox cases are represented instead of only a minimal subset.
- `golden_data/diagrams/w3c_master` adds W3C SVG 2 reference images across coords, filters, painting, paths, shapes, and struct categories so diagram coverage is not limited to one authoring-example subgroup.
- `golden_data/illustrations/openmoji` adds a curated OpenMoji color-SVG slice so illustration coverage is no longer dominated by the existing mixed-source root set.
- Golden SVG references are rasterized at a minimum longest-side resolution of `1024px` during `benchmark-golden` runs to preserve enough signal for edge and topology metrics.
- `benchmark-golden` now clears stale artifacts inside `artifacts/rendered_inputs` and `artifacts/vectorized` before each run so outdated PNGs and SVGs cannot pollute new reports.
- When a `limit` is provided, the benchmark uses deterministic balanced sampling across top-level golden groups instead of truncating the lexically first files.
- Limited golden runs now also spread samples across source-complexity bins and source subgroups inside each top-level group so the smaller suites still include easy, hard, and structurally different references.
- The previous legacy icon corpus is backed up under `benchmark_runs/dataset_backups/icons_legacy_feather`.

## Library Access

The same report is available through Rust and the SDK contract:

- Rust: `benchmark_directory`, `benchmark_golden_data`
- SDK layer: `benchmark`, `benchmark_golden`
