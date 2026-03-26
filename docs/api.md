# Rust API

Vectalab is a library first and a CLI second. The public API is intentionally compact.

## Main Entry Points

### `vectorize`

Convert one raster input into an SVG string plus a structured report.

```rust
use std::path::Path;
use vectalab::{vectorize, VectorizeOptions};

let (svg, report) = vectorize(Path::new("examples/test_logo_benchmark.png"), &VectorizeOptions::default())?;
println!("ssim={:.4}", report.metrics.ssim);
# Ok::<(), anyhow::Error>(())
```

### `VectorizeOptions`

Tune the conversion loop.

```rust
use vectalab::{QualityPreset, VectorizeOptions};

let options = VectorizeOptions {
    target_ssim: 0.95,
    max_file_size: 150_000,
    max_iterations: 4,
    quality: Some(QualityPreset::Balanced),
};
```

### `write_svg`

Persist generated SVG content to disk.

```rust
use std::path::Path;
use vectalab::write_svg;

write_svg(Path::new("out.svg"), "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>")?;
# Ok::<(), anyhow::Error>(())
```

### `analyze_image`

Inspect an image before conversion.

```rust
let image = image::open("examples/test_logo_benchmark.png")?;
let analysis = vectalab::analyze_image(&image);
println!("{:?}", analysis.image_type);
# Ok::<(), anyhow::Error>(())
```

### `compute_metrics`

Score an SVG against a raster source.

```rust
let image = image::open("examples/test_logo_benchmark.png")?;
let svg = std::fs::read_to_string("demo/output.svg")?;
let metrics = vectalab::compute_metrics(&image, &svg)?;
println!("paths={}", metrics.path_count);
# Ok::<(), anyhow::Error>(())
```

### `benchmark_directory`

Batch-process a directory and collect a report.

```rust
use std::path::Path;
use vectalab::{benchmark_directory, VectorizeOptions};

let report = benchmark_directory(
    Path::new("examples"),
    Path::new("benchmark_runs/latest"),
    &VectorizeOptions::default(),
)?;
println!("entries={}", report.entries.len());
# Ok::<(), anyhow::Error>(())
```

## Data Types

- `ImageAnalysis`: image dimensions, dominant colors, type, complexity, and heuristics
- `QualityMetrics`: `ssim`, `psnr`, `mae`, file size, and path count
- `VectorizationReport`: analysis, chosen trace settings, preset, and final metrics
- `BenchmarkReport`: per-file entries plus aggregate averages

## Stability

The crate is still early, but the goal is to keep the public API narrow so it can stabilize quickly.
