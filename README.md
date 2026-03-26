# EdgeSVG

[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

EdgeSVG is a native Rust CLI and library for turning raster graphics into compact, high-fidelity SVGs.

The goal is simple: make PNG and JPG to vector conversion fast, reproducible, scriptable, and good enough that open-source users stop needing a black-box commercial tool for the common cases.

## Why It Exists

Most raster-to-vector tooling falls into one of two traps:

- It is fast, but the output is noisy, oversized, or hard to trust.
- It is quality-focused, but wrapped in a heavy stack that is difficult to install, test, and automate.

EdgeSVG takes the practical middle ground:

- native Rust runtime
- in-process tracing via the vendored internal vectorizer
- adaptive preprocessing by image type
- smart, premium, and competitive auto flows modeled after the Python `vectalab` reference
- built-in SVG rendering and scoring
- reproducible benchmarks and CLI workflows

## What It Does

- Detects whether an input behaves more like a logo, icon, illustration, or photo
- Reduces palette complexity before tracing to keep SVGs clean
- Iterates through an OODA-style conversion loop until fidelity and size targets are balanced
- Scores candidates with built-in metrics including `ssim`, `psnr`, `mae`, file size, and path count
- Renders generated SVGs back to PNG for verification
- Benchmarks folders of assets and emits both JSON and Markdown reports

## Results

Local benchmark snapshot on the repository `examples/` set:

| Metric | Value |
|---|---:|
| Average SSIM | 0.9765 |
| Average PSNR | 28.17 |
| Average MAE | 2.62 |
| Average SVG Size | 17.4 KB |
| Average Path Count | 27.1 |

Representative sample:

- `examples/test_logo_benchmark.png` converts to a `4.7 KB` SVG at `0.9933` SSIM

## Quick Start

```bash
cargo build --release

# Convert a raster image to SVG
cargo run --release -- convert examples/test_logo_benchmark.png out.svg

# Run the adaptive smart strategy
cargo run --release -- smart examples/test_logo_benchmark.png out.smart.svg

# Inspect the detected image profile
cargo run --release -- analyze examples/test_logo_benchmark.png

# Compare a raster input against a generated SVG
cargo run --release -- compare examples/test_logo_benchmark.png out.svg

# Render the SVG back to PNG
cargo run --release -- render out.svg out.png

# Benchmark a directory of raster inputs
cargo run --release -- benchmark \
  --input-dir examples \
  --output-dir benchmark_runs/latest \
  --json-path benchmark_runs/latest/report.json \
  --markdown-path benchmark_runs/latest/report.md
```

## Install

### From Source

```bash
cargo install --path .
```

### As a Library

```toml
[dependencies]
edgesvg = "0.2"
```

```rust
use std::path::Path;
use edgesvg::{vectorize, VectorizeOptions};

let (svg, report) = vectorize(Path::new("examples/test_logo_benchmark.png"), &VectorizeOptions::default())?;
println!("ssim={:.4} paths={}", report.metrics.ssim, report.metrics.path_count);
# Ok::<(), anyhow::Error>(())
```

## CLI Overview

| Command | Purpose |
|---|---|
| `convert` | Convert one raster image to SVG |
| `smart` | Run the adaptive multi-pass native strategy search |
| `info` | Inspect file metadata plus the recommended conversion strategy |
| `analyze` | Inspect how EdgeSVG classifies the image |
| `compare` | Score a raster input against an SVG output |
| `render` | Render SVG to PNG for previewing |
| `optimize` | Minify an existing SVG with the built-in optimizer |
| `benchmark` | Batch-convert a directory and emit reports |
| `benchmark-golden` | Reproduce the imported golden-data benchmark corpus |

Detailed usage lives in [docs/cli.md](docs/cli.md).

## Architecture

The shipped pipeline is intentionally narrow and auditable:

1. Load the raster image.
2. Analyze color coverage, color variance, and edge density.
3. Choose preprocessing and tracing settings from the detected image class.
4. Quantize and lightly denoise before tracing.
5. Trace with the internal Rust vectorizer.
6. Minify path data without breaking SVG structure.
7. Render the SVG back and compute quality metrics.
8. Keep the best candidate under the configured constraints.

The implementation is in:

- [src/analysis.rs](src/analysis.rs)
- [src/preprocess.rs](src/preprocess.rs)
- [src/pipeline.rs](src/pipeline.rs)
- [src/highlevel.rs](src/highlevel.rs)
- [src/vectorizer.rs](src/vectorizer.rs)
- [src/svg.rs](src/svg.rs)
- [src/metrics.rs](src/metrics.rs)
- [src/benchmark.rs](src/benchmark.rs)

## Test Strategy

EdgeSVG ships three test layers:

- Unit tests inside core modules for classification, preprocessing, and SVG minification behavior
- Integration tests in `tests/library_api.rs` for the public library surface
- End-to-end CLI tests in `tests/cli_e2e.rs` covering `convert`, `smart`, `analyze`, `compare`, `render`, `optimize`, and `benchmark`

Run everything with:

```bash
cargo test
```

## Repository Layout

- `src/` core library and CLI
- `tests/` integration and e2e coverage
- `examples/` benchmark and demo assets
- `docs/` concise reference documentation
- `demo/` local walkthrough

## Open Source Standards

This repository is structured as a publishable Rust crate:

- root-level Cargo project
- committed `Cargo.lock`
- release-oriented metadata
- reproducible tests
- benchmarkable examples
- MIT license

## Roadmap

- Better shape-aware SVG simplification without fidelity loss
- More representative public benchmark datasets
- Optional machine-readable quality gates in CI
- Broader corpus for photo-heavy vectorization evaluation

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
