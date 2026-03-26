# Benchmark Runs

This directory keeps committed benchmark summaries for reproducible verification runs.

Current committed reference report:

- `golden_full/report.json`
- `golden_full/report.md`

Recommended workflows:

```bash
make bench-smoke
make bench-sample
make bench-full
make optimize-frontier
```

Equivalent direct generation command:

```bash
cargo run -- benchmark-golden \
  --golden-dir golden_data \
  --work-dir benchmark_runs/golden_full \
  --quality figma \
  --json-path benchmark_runs/golden_full/report.json \
  --markdown-path benchmark_runs/golden_full/report.md \
  --max-iterations 4
```

The benchmark JSON now includes:

- overall quality and compactness metrics
- per-group summaries
- average elapsed time and throughput
- per-entry group and reference identifiers

The large rendered/vectorized intermediate artifacts remain ignored to keep the repository size under control.
