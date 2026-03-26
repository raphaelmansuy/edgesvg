# Algorithm

Vectalab uses a deliberately pragmatic pipeline.

## Conversion Flow

1. Load the raster source.
2. Compute image heuristics:
   - unique color count
   - top color coverage
   - color variance
   - edge density
3. Classify the image as one of:
   - logo
   - icon
   - illustration
   - photo
4. Choose preprocessing and tracing settings from that class.
5. Reduce palette complexity and apply light denoising where it helps.
6. Trace with `vtracer`.
7. Minify SVG path data without collapsing document structure.
8. Render the SVG back to pixels and compute quality metrics.
9. Keep the best candidate under the configured fidelity and file size constraints.

## Why This Shape

The project does not try to be magical. It tries to be auditable:

- every stage is native Rust
- the output can be rendered and checked locally
- candidate selection is metric-driven
- the heuristics are intentionally readable in source

## Core Modules

- `src/analysis.rs`
- `src/preprocess.rs`
- `src/pipeline.rs`
- `src/svg.rs`
- `src/metrics.rs`
- `src/benchmark.rs`
