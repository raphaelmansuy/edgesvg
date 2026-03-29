---
title: Algorithm
description: How EdgeSVG vectorizes raster images — analysis, classification, mode selection, tracing, scoring, and the full pipeline.
---

EdgeSVG is a **scored search pipeline**, not a single tracing pass. The key design insight is: don't just convert and hope — *measure the result* and feed that measurement back into the conversion strategy.

## Pipeline Overview

```
Input raster (PNG / JPG / WebP)
       │
       ▼
1. Decode raster
       │
       ▼
2. Flatten transparency for analysis
   (preserves alpha-aware tracing where useful)
       │
       ▼
3. Image analysis
   - Width, height, aspect ratio
   - Color coverage and unique color count
   - Pixel variance
   - Edge density
   - Dominant color palette
       │
       ▼
4. Image classification
   → logo | icon | illustration | photo
       │
       ▼
5. Mode selection
   (auto picks from classification;
    smart runs iterative search)
       │
       ▼
6. Preprocessing
   (denoise, threshold, channel isolation)
       │
       ▼
7. SVG tracing
   (one or more quality presets per mode)
       │
       ▼
8. SVG optimization
   (coordinate precision, styling noise removal)
       │
       ▼
9. Re-rasterize SVG for scoring
       │
       ▼
10. Score against original raster
    (SSIM, edge F1, IoU, color, topology, ...)
       │
       ▼
If score ≥ target → return result
If score < target & iterations remain → try next preset
       │
       ▼
Return best candidate (or highest-scoring fallback)
```

## Image Classification

The classifier analyzes a set of image features and assigns one of four kinds:

| Kind | Characteristics |
|------|-----------------|
| `logo` | Monochrome or limited palette, transparent background, clean geometric shapes |
| `icon` | Small, simple, icon-like proportions |
| `illustration` | Richer content but still vector-friendly |
| `photo` | Complex photographic content |

This classification drives the default preprocessing and tracing presets. For example, a logo typically gets path-clean tracing with minimal colors, while an illustration gets a richer multi-pass strategy.

## Mode Selection

### `auto`

Reads the image classification and routes to the best mode:

- Monochrome transparent icons → logo path
- Richer images → premium path
- In-between cases → heuristic fallback

### `smart`

Runs an iterative quality search. For each iteration:

1. Try the next candidate strategy
2. Compute quality metrics
3. If `ssim ≥ target_ssim` AND `file_size ≤ max_file_size` → return
4. Otherwise continue to the next candidate

Returns the first candidate that hits both thresholds, or the highest-scoring result after the budget is exhausted.

### `logo`

Optimized for logos, icons, and clean geometric shapes. Targets clean minimal paths with correct closure, minimal node count, and accurate curves.

### `premium`

Higher-fidelity multi-pass strategy for richer images. Uses more colors and path segments to reproduce complex gradients and shapes.

## Scoring

After tracing, the SVG is re-rasterized and compared pixel-by-pixel (and structurally) against the original. The scoring layer produces:

- **Structural metrics**: SSIM, PSNR, MAE, perceptual SSIM
- **Edge metrics**: edge similarity, precision, recall, F1
- **Coverage metrics**: foreground IoU
- **Color metrics**: color similarity, delta-E
- **Complexity metrics**: path count, weighted path count, topology score
- **Size metrics**: file size in bytes

See [Quality Metrics](/concepts/quality-metrics/) for details on every metric.

## Why the SDK Contract Matters

All SDKs (CLI, Python, Node.js, WASM) call the same `VectorizeRequest` → `VectorizeResponse` layer. This means:

- CLI and SDKs agree on methods, defaults, and output structure
- Regression tests target one contract instead of four divergent ones
- Release packaging stays thin — no per-SDK behavior drift

## Fallback Behavior

If the requested mode fails (e.g., the tracer returns an empty SVG), EdgeSVG:

1. Falls back to the next best mode
2. Sets `fallback_from` in the response to explain the substitution
3. Returns the fallback result with the original quality metrics

The `decision` field in the response always explains what happened and why.
