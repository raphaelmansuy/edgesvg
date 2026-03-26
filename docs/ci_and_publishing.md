# CI And Publishing

## CI Matrix

GitHub Actions validates four surfaces:

| Workflow job | What it checks |
|---|---|
| `rust` | `cargo fmt`, `cargo clippy`, `cargo test` |
| `python` | `maturin develop` and `pytest` |
| `node` | `npm ci`, native addon build, TypeScript build, Vitest |
| `wasm` | `cargo check -p edgesvg-wasm --target wasm32-unknown-unknown` |

## Release Workflows

### Rust

`.github/workflows/release-rust.yml`

- triggers on `v*.*.*` tags
- runs `cargo publish`

### Python

`.github/workflows/release-python.yml`

- builds wheels on Linux, macOS, and Windows
- publishes to PyPI with `twine`

### Node.js

`.github/workflows/release-node.yml`

- builds platform-specific `.node` binaries
- publishes per-platform npm packages
- publishes the main `edgesvg` wrapper package

### WASM

`.github/workflows/release-wasm.yml`

- runs `wasm-pack build`
- uploads the generated package artifact

## Local Reproduction

```bash
make verify
make python-sdk
make node-sdk
make wasm-sdk
make verify-all
```

## Required Secrets

| Secret | Used by |
|---|---|
| `CARGO_REGISTRY_TOKEN` | crates.io publish |
| `PYPI_API_TOKEN` | PyPI publish |
| `NPM_TOKEN` | npm publish |
