# Algorithm

EdgeSVG is a scored search pipeline, not a single tracing pass.

## Pipeline

1. Decode the raster input.
2. Flatten transparency for analysis while preserving alpha-aware tracing where useful.
3. Measure image width, height, color coverage, variance, edge density, and dominant colors.
4. Classify the input as `logo`, `icon`, `illustration`, or `photo`.
5. Choose preprocessing and tracing presets from that classification.
6. Trace candidate SVGs through one or more quality presets.
7. Optimize SVG coordinate precision and styling noise.
8. Render the SVG back to raster.
9. Score the SVG against the original raster.
10. Return the first candidate that hits the target, or the highest-scoring fallback.

## Metrics

The `QualityMetrics` report includes:

- `ssim`
- `ssim_perceptual`
- `edge_similarity`
- `edge_precision`
- `edge_recall`
- `edge_f1`
- `foreground_iou`
- `color_similarity`
- `fidelity_score`
- `delta_e`
- `topology_score`
- `psnr`
- `mae`
- `file_size`
- `path_count` (raw `<path>` elements)
- `primitive_count` (native shapes such as `rect`, `circle`, `ellipse`, `line`, `polygon`, `polyline`)
- `weighted_path_count` (primitive-aware structural complexity used for editability-sensitive scoring)

## Mode Selection

`auto` and the SDK `decision` field are driven by:

- monochrome transparent icon detection
- image kind classification
- unique-color count
- top-color coverage

Small clean logos tend to route to the logo path. Richer images tend to route to the premium path.

## Why the SDK Contract Matters

All wrappers call the same `VectorizeRequest` and `VectorizeResponse` layer. That means:

- the CLI and SDKs agree on methods and defaults
- release packaging can stay thin
- regression tests target one contract instead of four divergent ones
