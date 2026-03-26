# Rust API

## Two Layers

EdgeSVG exposes:

1. The original Rust-native functions such as `vectorize`, `vectorize_auto`, and `benchmark_directory`
2. A stable SDK-facing contract in `src/sdk.rs` used by Python, Node.js, and WASM

## Core Functions

```rust
use std::path::Path;
use edgesvg::{vectorize, VectorizeOptions};

let (svg, report) = vectorize(
    Path::new("examples/test_logo_benchmark.png"),
    &VectorizeOptions::default(),
)?;
# Ok::<(), anyhow::Error>(())
```

Other core exports:

- `analyze_image`
- `vectorize_logo_premium`
- `vectorize_premium`
- `vectorize_auto`
- `vectorize_smart`
- `vectorize_optimal`
- `compute_metrics`
- `render_svg_to_image`
- `benchmark_directory`
- `benchmark_golden_data`

## Stable SDK Contract

### Request

```rust
use edgesvg::{VectorizeMethod, VectorizeRequest};

let request = VectorizeRequest {
    method: VectorizeMethod::Auto,
    target_ssim: 0.998,
    max_file_size: 100_000,
    max_iterations: 4,
    quality: edgesvg::QualityPreset::Ultra,
    logo_quality: None,
    colors: None,
};
```

### Response

`vectorize_path` and `vectorize_bytes` return:

- `svg`
- `report`
- `requested_method`
- `effective_method`
- `fallback_from`
- `decision`

### Functions

| Function | Input | Output |
|---|---|---|
| `vectorize_path` | file path + `VectorizeRequest` | `VectorizeResponse` |
| `vectorize_bytes` | raster bytes + `VectorizeRequest` | `VectorizeResponse` |
| `analyze_path` / `analyze_bytes` | raster | `AnalyzeResponse` |
| `inspect_path` | file path | `InfoResponse` |
| `compare_path` / `compare_bytes` | raster + SVG | `QualityMetrics` |
| `optimize` | SVG string | `OptimizeResponse` |
| `render_png` | SVG + width + height | PNG bytes |
| `benchmark` | directory paths + `BenchmarkRequest` | `BenchmarkReport` |
| `benchmark_golden` | golden corpus + work dir | `BenchmarkReport` |

## In-Memory APIs

WASM support required true in-memory conversion, so the Rust crate now also exposes:

- `vectorize_image`
- `determine_auto_mode_image`
- `vectorize_auto_image`
- `vectorize_logo_premium_image`
- `vectorize_premium_image`
- `vectorize_smart_image`
- `vectorize_optimal_image`

These are the right entrypoints for server-side pipelines that already decoded image data.
