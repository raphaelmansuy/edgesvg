# EdgeSVG

[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**Raster to SVG vectorization with one Rust engine and five delivery surfaces: library, CLI, Python, Node.js, and WebAssembly.**

EdgeSVG converts PNG and JPG inputs into compact SVGs while measuring fidelity against the original raster. The repository is structured so every packaging target reuses the same Rust decision logic, scoring, and output contract.

## Table of Contents

- [Why EdgeSVG](#why-edgesvg)
- [What Ships](#what-ships)
- [Quick Start](#quick-start)
- [CLI](#cli)
- [Rust API](#rust-api)
- [Python SDK](#python-sdk)
- [Node.js SDK](#nodejs-sdk)
- [WebAssembly](#webassembly)
- [Benchmarking](#benchmarking)
- [Repository Layout](#repository-layout)
- [CI/CD](#cicd)
- [Documentation](#documentation)

## Why EdgeSVG

Most open raster-to-vector tools have one of three problems:

- they are easy to install but produce noisy SVGs
- they produce decent SVGs but are hard to automate
- every SDK or wrapper drifts from the core engine

EdgeSVG is built to avoid that split:

- native Rust pipeline
- adaptive image analysis before tracing
- built-in SVG optimization and raster-vs-vector scoring
- deterministic CLI and library behavior
- Python, Node.js, and WASM bindings generated from the same Rust contract

## What Ships

| Surface | Status | Path |
|---|---|---|
| Rust crate + binary | Shipped | `Cargo.toml`, `src/` |
| CLI | Shipped | `cargo run -- ...` or installed binary |
| Python SDK | Shipped | `sdks/python` |
| Node.js SDK | Shipped | `sdks/node` |
| WebAssembly crate | Shipped | `crates/edgesvg-wasm` |
| CI/CD workflows | Shipped | `.github/workflows/` |

## Quick Start

### Local source build

```bash
cargo build --release
cargo run --release -- convert examples/test_logo_benchmark.png out.svg
```

### Local Python SDK

```bash
cd sdks/python
maturin develop
python -c "import edgesvg; print(edgesvg.version())"
```

### Local Node.js SDK

```bash
cd sdks/node
npm ci
npm run build
node -e "const sdk=require('./dist/index.js'); console.log(sdk.version())"
```

### Local WASM package

```bash
cargo check -p edgesvg-wasm --target wasm32-unknown-unknown
wasm-pack build crates/edgesvg-wasm --target bundler --out-dir pkg
```

## CLI

The primary command is `edgesvg`.

```bash
edgesvg convert input.png output.svg --method auto --json
```

Key subcommands:

| Command | Purpose |
|---|---|
| `convert` | Single-file raster to SVG conversion |
| `logo` | Logo-oriented tracing path |
| `premium` | Higher-fidelity adaptive path |
| `auto` | Let EdgeSVG choose logo vs premium |
| `smart` | Multi-pass target-driven search |
| `info` | Metadata plus recommended method |
| `analyze` | Raw image classification details |
| `compare` | Score a raster against an SVG |
| `render` | Render SVG to PNG |
| `optimize` | Minify an SVG |
| `benchmark` | Batch benchmark an input directory |
| `benchmark-golden` | Benchmark the golden SVG corpus |

Detailed command examples: [docs/cli.md](docs/cli.md)

## Rust API

The library exposes both low-level and package-friendly APIs.

Low-level:

```rust
use std::path::Path;
use edgesvg::{vectorize, VectorizeOptions};

let (svg, report) = vectorize(
    Path::new("examples/test_logo_benchmark.png"),
    &VectorizeOptions::default(),
)?;
println!("ssim={:.4}", report.metrics.ssim);
# Ok::<(), anyhow::Error>(())
```

Stable SDK-facing contract:

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
println!("{}", result.svg.len());
# Ok::<(), anyhow::Error>(())
```

More: [docs/api.md](docs/api.md)

## Python SDK

```python
import edgesvg

result = edgesvg.vectorize("examples/test_logo_benchmark.png", method="auto")
print(result["report"]["metrics"]["ssim"])

metrics = edgesvg.compare("examples/test_logo_benchmark.png", result["svg"])
preview = edgesvg.render_png(result["svg"], 512, 512)
```

Package docs: [docs/python_sdk.md](docs/python_sdk.md)

## Node.js SDK

```ts
import { compare, renderPng, vectorize } from 'edgesvg';

const result = vectorize('examples/test_logo_benchmark.png', { method: 'auto' });
const metrics = compare('examples/test_logo_benchmark.png', result.svg);
const preview = renderPng(result.svg, 512, 512);
```

Package docs: [docs/node_sdk.md](docs/node_sdk.md)

## WebAssembly

The WASM crate accepts raster bytes and returns serializable objects, which makes it suitable for browser upload flows and serverless edge runtimes.

```ts
import init, { vectorize } from './pkg/edgesvg_wasm.js';

await init();
const result = vectorize(fileBytes, { method: 'auto' });
```

More: [docs/wasm_sdk.md](docs/wasm_sdk.md)

## Benchmarking

Fast local verification:

```bash
make bench-smoke
```

Main regression sweep:

```bash
make bench-sample
```

Full corpus:

```bash
make bench-full
```

More: [docs/benchmarks.md](docs/benchmarks.md)

## Repository Layout

| Path | Purpose |
|---|---|
| `src/` | Rust library and CLI |
| `crates/edgesvg-python` | PyO3 native module |
| `crates/edgesvg-node` | N-API native addon |
| `crates/edgesvg-wasm` | `wasm-bindgen` wrapper |
| `sdks/python` | Python package metadata and tests |
| `sdks/node` | Node package metadata and tests |
| `docs/` | High-signal operational docs |
| `tests/` | Integration and CLI tests |
| `golden_data/` | Regression benchmark corpus |

## CI/CD

GitHub Actions now validates:

- Rust formatting, clippy, and tests
- Python SDK build and tests
- Node.js SDK build and tests
- WASM target compilation

Release workflows are included for:

- crates.io publishing
- PyPI publishing
- npm publishing with platform-specific native packages
- WASM package artifact generation

Details: [docs/ci_and_publishing.md](docs/ci_and_publishing.md)

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

## License

MIT. See [LICENSE](LICENSE).
