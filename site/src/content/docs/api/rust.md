---
title: Rust API Reference
description: Complete reference for the EdgeSVG Rust crate — the core library used by all other SDKs.
---

import { Code } from '@astrojs/starlight/components';

The `edgesvg` Rust crate is the core engine. All other SDKs (Python, Node.js, WASM) are thin wrappers around it.

## Installation

```toml
[dependencies]
edgesvg = "0.1"
```

Or with Cargo:

```bash
cargo add edgesvg
```

## Main Entry Points

### `vectorize_file`

Convert an image file to SVG.

```rust
use edgesvg::{vectorize_file, VectorizationConfig};

let config = VectorizationConfig::default();
let response = vectorize_file("input.png", Some("output.svg"), &config)?;

println!("SSIM: {:.4}", response.report.metrics.ssim);
println!("Paths: {}", response.report.metrics.path_count);
```

### `vectorize_bytes`

Convert image bytes in memory (used by WASM and FFI bindings).

```rust
use edgesvg::{vectorize_bytes, VectorizationConfig};

let image_bytes: Vec<u8> = std::fs::read("input.png")?;
let config = VectorizationConfig::default();
let response = vectorize_bytes(&image_bytes, &config)?;

let svg_string = response.svg;
```

## Configuration

### `VectorizationConfig`

```rust
use edgesvg::{VectorizationConfig, VectorizeMethod};

let config = VectorizationConfig {
    method: VectorizeMethod::Auto,
    target_ssim: Some(0.995),
    max_file_size: Some(512 * 1024), // 512 KB
    output_path: Some("output.svg".into()),
    json: false,
    verbose: false,
    ..Default::default()
};
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `method` | `VectorizeMethod` | `Auto` | Vectorization strategy |
| `target_ssim` | `Option<f64>` | `None` | Target SSIM for smart mode |
| `max_file_size` | `Option<u64>` | `None` | Max output size in bytes |
| `output_path` | `Option<PathBuf>` | `None` | Output SVG path (None = in-memory) |
| `json` | `bool` | `false` | Print JSON report to stdout |
| `verbose` | `bool` | `false` | Full verbose output |

### `VectorizeMethod`

```rust
pub enum VectorizeMethod {
    Auto,
    Logo,
    Premium,
    Smart,
    Convert,
}
```

Parse from string:

```rust
let method: VectorizeMethod = "logo".parse()?;
```

## Response Types

### `VectorizeResponse`

```rust
pub struct VectorizeResponse {
    pub svg: String,                    // The generated SVG markup
    pub report: QualityReport,          // Metrics and scoring
    pub requested_method: VectorizeMethod,
    pub effective_method: VectorizeMethod,
    pub fallback_from: Option<VectorizeMethod>,
    pub decision: Option<String>,       // Human-readable explanation (auto mode)
}
```

### `QualityReport`

```rust
pub struct QualityReport {
    pub metrics: QualityMetrics,
    pub score: f64,                     // 0.0 – 1.0 composite score
    pub grade: String,                  // "A+", "A", "B", etc.
    pub summary: String,
}
```

### `QualityMetrics`

```rust
pub struct QualityMetrics {
    pub ssim: f64,
    pub ssim_perceptual: f64,
    pub edge_similarity: f64,
    pub edge_precision: f64,
    pub edge_recall: f64,
    pub edge_f1: f64,
    pub foreground_iou: f64,
    pub color_similarity: f64,
    pub fidelity_score: f64,
    pub delta_e: f64,
    pub topology_score: f64,
    pub psnr: f64,
    pub mae: f64,
    pub file_size: u64,
    pub path_count: u32,
    pub weighted_path_count: f64,
}
```

## Error Handling

All functions return `Result<_, edgesvg::Error>`:

```rust
use edgesvg::Error;

match vectorize_file("input.png", None, &config) {
    Ok(response) => println!("{}", response.svg),
    Err(Error::Io(e)) => eprintln!("File error: {e}"),
    Err(Error::Decode(e)) => eprintln!("Image decode error: {e}"),
    Err(Error::Vectorize(e)) => eprintln!("Vectorization failed: {e}"),
    Err(e) => eprintln!("Other error: {e}"),
}
```

## Examples

### Batch processing

```rust
use edgesvg::{vectorize_file, VectorizationConfig, VectorizeMethod};
use std::path::Path;

fn batch_convert(inputs: &[&str], out_dir: &str) -> anyhow::Result<()> {
    let config = VectorizationConfig {
        method: VectorizeMethod::Auto,
        ..Default::default()
    };

    for input in inputs {
        let stem = Path::new(input).file_stem().unwrap().to_str().unwrap();
        let output = format!("{}/{}.svg", out_dir, stem);
        let response = vectorize_file(input, Some(&output), &config)?;
        println!("{} → {} (SSIM={:.4})", input, output, response.report.metrics.ssim);
    }
    Ok(())
}
```

### Quality gating

```rust
fn convert_with_quality_check(path: &str) -> anyhow::Result<()> {
    let config = VectorizationConfig {
        method: VectorizeMethod::Smart,
        target_ssim: Some(0.995),
        ..Default::default()
    };

    let response = vectorize_file(path, None, &config)?;

    if response.report.metrics.ssim < 0.99 {
        anyhow::bail!("SSIM {:.4} below threshold", response.report.metrics.ssim);
    }

    println!("Grade: {}", response.report.grade);
    Ok(())
}
```
