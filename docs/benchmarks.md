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
- `benchmark_runs/golden_full_current`
- `benchmark_runs/optimization_frontier`

## Library Access

The same report is available through Rust and the SDK contract:

- Rust: `benchmark_directory`, `benchmark_golden_data`
- SDK layer: `benchmark`, `benchmark_golden`
