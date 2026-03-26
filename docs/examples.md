# Examples

## CLI

Convert a single image:

```bash
cargo run --release -- convert examples/test_logo_benchmark.png out.svg
```

Analyze before converting:

```bash
cargo run --release -- analyze examples/test_logo_benchmark.png --json
```

Render back to PNG:

```bash
cargo run --release -- render out.svg out.png
```

Benchmark a directory:

```bash
cargo run --release -- benchmark \
  --input-dir examples \
  --output-dir benchmark_runs/latest \
  --json-path benchmark_runs/latest/report.json \
  --markdown-path benchmark_runs/latest/report.md
```

## Library

```rust
use std::path::Path;
use edgesvg::{vectorize, write_svg, VectorizeOptions};

let (svg, report) = vectorize(
    Path::new("examples/test_logo_benchmark.png"),
    &VectorizeOptions::default(),
)?;

write_svg(Path::new("out.svg"), &svg)?;
println!("{:.4}", report.metrics.ssim);
# Ok::<(), anyhow::Error>(())
```
