# EdgeSVG

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)

**Open source raster-to-SVG vectorization that is actually built for production automation, not for demos.**

Most teams that need vectorization in a build pipeline, design-export flow, or asset ingestion service run into the same problem:

- commercial tools produce strong output, but they are black boxes, expensive, and hard to automate
- classic open-source tracers are easy to call, but the SVGs are noisy, oversized, or brittle
- language wrappers around native tracers usually drift, so the CLI, Python package, browser build, and Node module behave differently

EdgeSVG exists to fix that.

It is a single Rust-native engine that:

- analyzes the input before tracing
- chooses different strategies for logos, icons, illustrations, and photos
- measures SVG quality against the original raster instead of pretending every run is “good”
- ships the same contract across CLI, Rust, Python, Node.js, and WebAssembly

If the goal is “convert PNG or JPG assets into SVGs inside a real software system,” this project is aimed at that exact use case.

## Why This Project Exists

The open-source gap is not “can we trace an image into paths.”

The gap is:

- reproducibility
- quality measurement
- packaging
- integration
- operational trust

That is why EdgeSVG is opinionated about the parts most repos avoid:

| Problem | EdgeSVG answer |
|---|---|
| The same image needs different handling depending on whether it is a logo or a rich illustration | Built-in image classification drives preprocessing and tracing decisions |
| SVG output may look acceptable but regress silently over time | Every run can be scored against the source raster with SSIM, edge similarity, IoU, topology, size, and path count |
| Teams need one engine across multiple runtimes | Rust is the source of truth; Python, Node.js, and WASM bind to the same contract |
| Batch conversion needs to be benchmarked and compared over time | The repo ships benchmark runners, golden data, JSON reports, and Markdown reports |
| Most repos claim multi-language support but only the Rust path is tested | CI validates Rust, Python, Node.js, and WASM separately |

## What EdgeSVG Is

EdgeSVG is:

- a Rust crate
- a CLI binary
- a Python SDK
- a Node.js SDK
- a WebAssembly package
- a benchmarkable conversion pipeline

EdgeSVG is not:

- a GUI design tool
- an OCR engine
- a hand-drawn illustration cleanup tool
- a hosted API

## Why It Is Different

The core design choice is simple: **do not expose tracing as a raw black-box call and hope callers clean up the result later**.

EdgeSVG performs:

1. raster analysis
2. mode selection
3. preprocessing
4. tracing
5. SVG optimization
6. rasterized verification
7. candidate scoring

That means the output contract already contains the information teams actually need:

- what method was requested
- what method was actually used
- what the image was classified as
- how good the result was
- how large the result is
- how complex the path structure is

## Product Surfaces

| Surface | Status | Notes |
|---|---|---|
| Rust | Ready | direct crate API plus stable SDK contract |
| CLI | Ready | machine-readable JSON and benchmark commands |
| Python | Ready | PyO3 native extension |
| Node.js | Ready | N-API native addon with platform packages |
| WASM | Ready | `wasm-bindgen` wrapper for browser and edge runtimes |

## Installation

### CLI from source

```bash
cargo install --path .
```

### Rust

```toml
[dependencies]
edgesvg = "0.2"
```

### Python

```bash
cd sdks/python
maturin develop
```

### Node.js

```bash
cd sdks/node
npm ci
npm run build
```

### WebAssembly

```bash
wasm-pack build crates/edgesvg-wasm --target bundler --out-dir pkg --release
```

## Quick Start

### CLI

```bash
edgesvg convert examples/test_logo_benchmark.png out.svg --method auto --json
```

### Rust

```rust
use std::path::Path;
use edgesvg::{vectorize_path, VectorizeMethod, VectorizeRequest};

let result = vectorize_path(
    Path::new("examples/test_logo_benchmark.png"),
    &VectorizeRequest {
        method: VectorizeMethod::Auto,
        ..VectorizeRequest::default()
    },
)?;

println!("ssim={:.4}", result.report.metrics.ssim);
# Ok::<(), anyhow::Error>(())
```

### Python

```python
import edgesvg

result = edgesvg.vectorize("examples/test_logo_benchmark.png", method="auto")
print(result["report"]["metrics"]["ssim"])
```

### Node.js

```ts
import { vectorize } from 'edgesvg';

const result = vectorize('examples/test_logo_benchmark.png', { method: 'auto' });
console.log(result.report.metrics.ssim);
```

### WebAssembly

```ts
import init, { vectorize } from './pkg/edgesvg_wasm.js';

await init();
const result = vectorize(fileBytes, { method: 'auto' });
```

## What You Get Back

The stable SDK-facing contract returns:

- `svg`
- `report`
- `requested_method`
- `effective_method`
- `fallback_from`
- `decision`

This is intentional. A vectorizer that only returns a string is hard to operate at scale.

## CLI Command Map

| Command | Purpose |
|---|---|
| `convert` | default single-file conversion |
| `logo` | logo and icon oriented conversion |
| `premium` | higher-fidelity conversion path |
| `auto` | let EdgeSVG choose the mode |
| `smart` | iterative target-driven search |
| `info` | metadata and recommendation |
| `analyze` | image classification details |
| `compare` | raster-vs-SVG metrics |
| `render` | SVG to PNG |
| `optimize` | SVG cleanup and minification |
| `benchmark` | batch raster benchmark |
| `benchmark-golden` | golden SVG benchmark |

## Benchmarking

EdgeSVG ships with a benchmark workflow because “looks good to me” is not a release criterion.

```bash
make bench-smoke
make bench-sample
make bench-full
```

Reports include:

- fidelity metrics
- size and path complexity
- per-group summaries
- lowest-performing assets
- JSON and Markdown outputs

## CI/CD

The repository validates the full product surface:

- Rust formatting, lint, and tests
- Python package build and tests
- Node.js addon build and tests
- WASM compilation and package build

Release workflows are included for:

- crates.io
- PyPI
- npm
- WASM package artifacts

## Repository Layout

| Path | Purpose |
|---|---|
| `src/` | Rust engine and CLI |
| `crates/edgesvg-python` | Python native binding crate |
| `crates/edgesvg-node` | Node native binding crate |
| `crates/edgesvg-wasm` | WASM wrapper crate |
| `sdks/python` | Python package metadata and tests |
| `sdks/node` | Node package metadata and tests |
| `docs/` | operational documentation |
| `tests/` | Rust integration and e2e tests |
| `golden_data/` | benchmark corpus |

## Documentation

- [docs/README.md](docs/README.md)
- [docs/cli.md](docs/cli.md)
- [docs/api.md](docs/api.md)
- [docs/python_sdk.md](docs/python_sdk.md)
- [docs/node_sdk.md](docs/node_sdk.md)
- [docs/wasm_sdk.md](docs/wasm_sdk.md)
- [docs/algorithm.md](docs/algorithm.md)
- [docs/benchmarks.md](docs/benchmarks.md)
- [docs/ci_and_publishing.md](docs/ci_and_publishing.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
