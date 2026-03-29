---
title: Rust Quick Start
description: Use EdgeSVG as a Rust crate. Direct API access to the vectorization engine with full quality scoring.
sidebar:
  order: 4
---

## Add Dependency

```toml
[dependencies]
edgesvg = "0.2"
# or for async pipelines:
# edgesvg = { version = "0.2", features = [] }
```

Requires Rust 1.82+.

## Basic Usage

### Stable SDK contract (recommended)

```rust
use std::path::Path;
use edgesvg::{vectorize_path, VectorizeMethod, VectorizeRequest};

fn main() -> anyhow::Result<()> {
    let result = vectorize_path(
        Path::new("logo.png"),
        &VectorizeRequest {
            method: VectorizeMethod::Auto,
            target_ssim: 0.998,
            max_file_size: 100_000,
            max_iterations: 4,
            ..VectorizeRequest::default()
        },
    )?;

    println!("SSIM:    {:.4}", result.report.metrics.ssim);
    println!("Edge F1: {:.4}", result.report.metrics.edge_f1);
    println!("Method:  {:?}", result.effective_method);
    println!("Decision: {}", result.decision);

    // Save SVG
    std::fs::write("logo.svg", &result.svg)?;
    Ok(())
}
```

### Vectorize from bytes (in-memory)

```rust
use edgesvg::{vectorize_bytes, VectorizeRequest};

let bytes = std::fs::read("logo.png")?;
let result = vectorize_bytes(&bytes, &VectorizeRequest::default())?;
println!("SVG length: {}", result.svg.len());
```

### Analyze an image

```rust
use edgesvg::{analyze_path, analyze_bytes};

// From file
let analysis = analyze_path(Path::new("logo.png"))?;
println!("Kind: {:?}", analysis.decision);

// From bytes
let bytes = std::fs::read("logo.png")?;
let analysis = analyze_bytes(&bytes)?;
```

### Compare SVG quality

```rust
use edgesvg::{compare_path, compare_bytes};

// From file
let metrics = compare_path(Path::new("logo.png"), &svg_string)?;
println!("SSIM: {:.4}", metrics.ssim);
println!("Edge F1: {:.4}", metrics.edge_f1);
println!("Fidelity: {:.4}", metrics.fidelity_score);

// From bytes
let png_bytes = std::fs::read("logo.png")?;
let metrics = compare_bytes(&png_bytes, &svg_string)?;
```

### Optimize SVG

```rust
use edgesvg::optimize;

let result = optimize(&svg_string, 2)?; // precision = 2 decimal places
println!("Optimized size: {}", result.svg.len());
```

### Render SVG to PNG

```rust
use edgesvg::render_png;

let png_bytes = render_png(&svg_string, 512, 512)?;
std::fs::write("preview.png", &png_bytes)?;
```

### Batch benchmark

```rust
use edgesvg::{benchmark, BenchmarkRequest};

let report = benchmark(
    &["images/"],
    "results/",
    &BenchmarkRequest::default(),
)?;
println!("Mean SSIM: {:.4}", report.summary.mean_ssim);
```

## Vectorization Methods

```rust
use edgesvg::VectorizeMethod;

let methods = [
    VectorizeMethod::Auto,      // EdgeSVG chooses
    VectorizeMethod::Logo,      // Clean paths for logos/icons
    VectorizeMethod::Premium,   // High-fidelity for complex images
    VectorizeMethod::Smart,     // Iterative search to hit SSIM target
];
```

## Request Parameters

```rust
use edgesvg::{VectorizeMethod, VectorizeRequest, QualityPreset};

let request = VectorizeRequest {
    method: VectorizeMethod::Auto,
    target_ssim: 0.998,           // target SSIM for smart mode
    max_file_size: 100_000,       // max SVG size in bytes
    max_iterations: 4,            // max iterations for smart mode
    quality: QualityPreset::Ultra,
    logo_quality: None,
    colors: None,                 // override color count
};
```

## Available Core Functions

| Function | Input | Output |
|----------|-------|--------|
| `vectorize_path` | file path + `VectorizeRequest` | `VectorizeResponse` |
| `vectorize_bytes` | raster bytes + `VectorizeRequest` | `VectorizeResponse` |
| `analyze_path` / `analyze_bytes` | raster | `AnalyzeResponse` |
| `inspect_path` | file path | `InfoResponse` |
| `compare_path` / `compare_bytes` | raster + SVG string | `QualityMetrics` |
| `optimize` | SVG string + precision | `OptimizeResponse` |
| `render_png` | SVG + width + height | `Vec<u8>` (PNG bytes) |
| `benchmark` | directory paths + `BenchmarkRequest` | `BenchmarkReport` |
| `benchmark_golden` | golden corpus + work dir | `BenchmarkReport` |

## Low-Level Access

For server-side pipelines that have already decoded image data:

```rust
use edgesvg::{
    vectorize_image,
    vectorize_auto_image,
    vectorize_logo_premium_image,
    vectorize_premium_image,
};
```

These accept decoded image buffers directly and are the right entrypoints for high-throughput pipelines.
