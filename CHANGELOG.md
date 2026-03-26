# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and the project follows Semantic Versioning.

## [Unreleased]

### Added

- GitHub Actions CI for Rust, Python, Node.js, and WASM
- release workflows for crates.io, PyPI, npm, and WASM packaging
- stable SDK-facing Rust request and response contract
- PyO3-based Python bindings and package layout
- N-API based Node.js bindings and package layout
- `wasm-bindgen` wrapper crate for in-browser and edge-runtime use
- focused SDK documentation for Python, Node.js, and WASM

### Changed

- README rewritten to lead with the problem, rationale, and product surface
- project license switched to Apache-2.0
- repository organized as a Rust workspace with language binding crates
- Makefile extended with package verification targets

## [0.2.0] - 2026-03-26

### Added

- adaptive Rust raster-to-SVG pipeline with CLI, benchmarking, metrics, and optimization flows
