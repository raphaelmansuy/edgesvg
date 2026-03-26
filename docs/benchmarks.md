# Benchmarks

Vectalab ships a native benchmark runner instead of the old Python scripts.

## Run

```bash
cargo run --release -- benchmark \
  --input-dir examples \
  --output-dir benchmark_runs/latest \
  --markdown-path benchmark_runs/latest/report.md \
  --json-path benchmark_runs/latest/report.json
```

## What It Produces

- one SVG per raster input in `--output-dir`
- a machine-readable JSON report
- a Markdown summary table

## Metrics

- `ssim`
- `psnr`
- `mae`
- SVG file size
- SVG path count

These are intentionally simple, reproducible quality checks that can run inside the Rust toolchain without Python or external services.

## Latest Snapshot

Latest local run on the repository `examples/` set:

| Metric | Value |
|---|---:|
| Average SSIM | 0.9765 |
| Average PSNR | 28.17 |
| Average MAE | 2.62 |
| Average SVG Size | 17.4 KB |
| Average Path Count | 27.1 |

Representative per-file results:

| Input | SSIM | Size (KB) | Paths |
|---|---:|---:|---:|
| `BEST_comparison.png` | 0.9878 | 23.8 | 39 |
| `ELITIZON_LOGO_8colors_reduced.png` | 0.9776 | 10.2 | 18 |
| `test_logo_benchmark.png` | 0.9933 | 4.7 | 6 |
