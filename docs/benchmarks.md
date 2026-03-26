# Benchmarks

EdgeSVG ships a native Rust benchmark runner plus thin Python wrappers for reproducible suites, report formatting, and OODA-style optimization loops.

## Recommended Commands

```bash
make verify
make bench-smoke
make bench-sample
make optimize-frontier
```

## Golden Corpus

```bash
make bench-full
```

## Direct Run

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
- group summaries
- lowest-fidelity entries for quick regression inspection

## Metrics

- `fidelity_score`
- `ssim`
- `ssim_perceptual`
- `edge_similarity`
- `edge_f1`
- `foreground_iou`
- `color_similarity`
- `topology_score`
- `psnr`
- `mae`
- SVG file size
- SVG path count
- elapsed time and throughput

These metrics are intentionally local and reproducible. The benchmark methodology is:

- render reference SVGs from `golden_data/` into PNG inputs
- vectorize them with `edgesvg`
- re-render the produced SVG
- compare rasterized output against the source PNG with structural and perceptual metrics
- rank optimization trials with a fidelity-first composite score, then compactness and runtime

## Current Snapshot

Latest smoke benchmark on the 12-asset golden slice:

| Metric | Value |
|---|---:|
| Average Fidelity | 0.8813 |
| Average SSIM | 0.8484 |
| Average Edge F1 | 0.9704 |
| Average Foreground IoU | 0.7966 |
| Average SVG Size | 0.8 KB |
| Average Path Count | 3.3 |

Best 90-asset optimization frontier point after 10 OODA loops:

| Metric | Value |
|---|---:|
| Fidelity | 0.8960 |
| SSIM | 0.8611 |
| Edge F1 | 0.9714 |
| Foreground IoU | 0.8456 |
| SVG Size | 1.4 KB |
| Path Count | 14.2 |
