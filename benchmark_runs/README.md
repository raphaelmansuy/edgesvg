# Benchmark Runs

This directory keeps committed benchmark summaries for reproducible verification runs.

Current committed report:

- `golden_full/report.json`
- `golden_full/report.md`

Generation command:

```bash
cargo run -- benchmark-golden \
  --golden-dir golden_data \
  --work-dir benchmark_runs/golden_full \
  --quality figma \
  --json-path benchmark_runs/golden_full/report.json \
  --markdown-path benchmark_runs/golden_full/report.md
```

The large rendered/vectorized intermediate artifacts remain ignored to keep the repository size under control.
