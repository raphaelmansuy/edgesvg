---
title: Quality Metrics
description: Every EdgeSVG output is scored with 15+ quality metrics. Learn what each metric measures and how to use them.
---

EdgeSVG computes a comprehensive quality report on every vectorization output. The metrics are grouped into structural, edge, coverage, color, complexity, and size categories.

## Viewing Metrics

### CLI

```bash
edgesvg convert logo.png logo.svg --json | jq '.report.metrics'
```

### Python

```python
result = edgesvg.vectorize("logo.png", method="auto")
metrics = result["report"]["metrics"]
print(metrics)
```

### Standalone comparison

```bash
# Compare any existing SVG against its raster source
edgesvg compare logo.png logo.svg --json
```

---

## Structural Metrics

### `ssim` — Structural Similarity Index

**Range:** 0.0–1.0 (higher is better)

SSIM models the human visual system by comparing luminance, contrast, and structure. A value above **0.99** indicates near-perfect fidelity.

```
ssim = 0.9987  → excellent
ssim = 0.9700  → good
ssim = 0.8500  → noticeable differences
```

### `ssim_perceptual` — Perceptual SSIM

SSIM computed in a perceptually uniform color space. More meaningful than raw SSIM for images with chroma variation.

### `psnr` — Peak Signal-to-Noise Ratio

**Unit:** dB (higher is better, typically 30–60 dB)

Measures the ratio between the maximum possible signal strength and the noise (difference). Above **40 dB** is generally considered high quality.

### `mae` — Mean Absolute Error

**Range:** 0.0–1.0 (lower is better)

Average per-pixel absolute difference between the rasterized SVG and the source. Complements SSIM for detecting subtle uniform color shifts.

---

## Edge Metrics

### `edge_similarity`

**Range:** 0.0–1.0 (higher is better)

How closely the SVG edges match the detected raster edges (Canny or Sobel edge map). Measures whether paths trace actual object boundaries.

### `edge_precision`

The fraction of SVG edge pixels that correspond to real edges in the source. High precision = few spurious paths.

### `edge_recall`

The fraction of real raster edges that are reproduced in the SVG. High recall = few missed boundaries.

### `edge_f1`

**Range:** 0.0–1.0 (higher is better)

Harmonic mean of edge precision and recall. The primary edge quality summary metric:

```
edge_f1 = 0.94  → excellent edge reproduction
edge_f1 = 0.80  → acceptable but some boundaries missed or added
edge_f1 < 0.70  → noticeable edge quality issues
```

---

## Coverage Metrics

### `foreground_iou` — Intersection over Union

**Range:** 0.0–1.0 (higher is better)

Measures how well the SVG covers the foreground of the source image. Computed as the intersection of foreground masks divided by their union.

Very useful for logos and icons where shape coverage completeness matters.

---

## Color Metrics

### `color_similarity`

**Range:** 0.0–1.0 (higher is better)

Compares the dominant color palettes of the SVG and source. A high value means the colors are reproduced accurately.

### `delta_e` — CIE 2000 Color Difference

**Range:** ≥ 0.0 (lower is better)

Delta-E 2000 is the perceptual color difference standard used in the printing and design industries:

```
delta_e < 1.0   → imperceptible difference
delta_e 1–2     → perceptible only to a trained eye
delta_e 2–10    → clearly visible difference
delta_e > 10    → very different colors
```

---

## Topology & Complexity Metrics

### `topology_score`

**Range:** 0.0–1.0 (higher is better)

Measures how well the SVG reproduces the structural topology of the source — shape count, connectivity, and nesting. Penalizes fragmented or over-merged paths.

### `path_count`

The raw number of `<path>` elements in the generated SVG. Fewer paths generally means cleaner, more editable output — but too few may miss fine details.

### `weighted_path_count`

A primitive-aware structural complexity score. Native shapes (`rect`, `circle`, `ellipse`, `line`, `polygon`, `polyline`) are weighted differently from generic `<path>` elements. Used as an editability metric — lower values indicate simpler-to-edit output.

---

## Size Metrics

### `file_size`

SVG file size in bytes. Smaller is generally better for web delivery and editing performance. Combined with `ssim`, this lets you make quality-vs-size trade-offs explicitly.

---

## The `fidelity_score` Composite

### `fidelity_score`

A weighted composite of the above metrics tuned for visual fidelity:

```
fidelity_score ≈ w₁·ssim + w₂·edge_f1 + w₃·foreground_iou + w₄·color_similarity + w₅·topology_score
```

Use `fidelity_score` as a single-number quality gate in CI:

```bash
FIDELITY=$(edgesvg compare logo.png logo.svg --json | jq '.fidelity_score')
python3 -c "import sys; sys.exit(0 if float('$FIDELITY') >= 0.95 else 1)"
```

---

## Using Metrics in CI

```yaml
# GitHub Actions quality gate
- name: Check SVG fidelity
  run: |
    edgesvg compare assets/logo.png dist/logo.svg --json > report.json
    SSIM=$(jq '.ssim' report.json)
    python3 -c "
    ssim = float('$SSIM')
    if ssim < 0.98:
        print(f'FAIL: SSIM {ssim:.4f} < 0.98')
        exit(1)
    print(f'PASS: SSIM {ssim:.4f}')
    "
```

## Benchmark Aggregates

When running `edgesvg benchmark`, metrics are aggregated per group and globally:

```json
{
  "summary": {
    "mean_ssim": 0.9923,
    "mean_edge_f1": 0.9156,
    "mean_foreground_iou": 0.9782,
    "mean_fidelity_score": 0.9634
  }
}
```
