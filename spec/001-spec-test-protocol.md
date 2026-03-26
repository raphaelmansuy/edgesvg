# SOTA Test and Improvement Protocol for Logos & Illustrations

## Overview
This protocol defines the rigorous process required to achieve **State-of-the-Art (SOTA)** vectorization for logos and illustrations. Unlike general photo vectorization, logos and flat illustrations require **geometric precision**, **clean topology**, and **exact color reproduction**.

Our goal is to produce SVGs that are indistinguishable from source files created in professional tools like Illustrator or Figma, suitable for production use in branding and web design.

## Quick Start

### Prerequisites
- Vectalab installed: `pip install -e .`
- Test data directories: `mkdir -p test_data/{svg_mono,svg_multi,png_mono,png_multi,vectalab_mono,vectalab_multi,reports}`

### Standard Workflow
```bash
# 1. Setup Test Data
python scripts/download_test_svgs.py
python scripts/convert_svg_to_png.py

# 2. Run SOTA Baseline (Logo Mode)
python scripts/quick_baseline.py --mode logo

# 3. Generate Comparison Report
python scripts/compare_results.py
```

## SOTA Test Suites

### 1. Geometric Precision Suite (The "Corner" Test)
- **Dataset**: 20 Feather Icons (Monochrome, Stroke-based)
- **Focus**: Sharp corners, consistent stroke widths, perfect circles.
- **SOTA Goal**: 
    - No rounded corners where sharp ones exist.
    - Perfect alignment with original paths.
    - **SSIM > 99.9%**

### 2. Brand Identity Suite (The "Color" Test)
- **Dataset**: 20 Font Awesome Brands + Complex Real-World Logos (e.g., Elitizon)
- **Focus**: Exact palette reproduction, separation of distinct colored regions.
- **SOTA Goal**: 
    - Zero "speckle" noise.
    - Exact color matches (ΔE < 1.0).
    - **Topology Score = 100%** (No missing holes in letters like 'A', 'O', 'e').

### 3. Illustration Suite (The "Layer" Test)
- **Dataset**: Flat Illustrations (UnDraw style) + W3C Complex Scenes
- **Focus**: Layer ordering, occlusion handling, and clean boundaries between shapes.
- **SOTA Goal**:
    - Clean separation of foreground/background.
    - Efficient path usage (no thousands of tiny polygons).
    - **Visual Fidelity > 99.9%**

## The SOTA Improvement Loop (OODA)

Follow this cycle to systematically eliminate artifacts.

### 1. Observe (Micro-Analysis)
Run the test suite and zoom in on the output.
```bash
python scripts/run_vectalab_test.py --mode logo
```
- **Check**: `test_data/baseline_report.txt`
- **Identify**: The bottom 3 performing assets.

### 2. Orient (Artifact Classification)
Identify specific non-SOTA artifacts:
- **The "Wobble"**: Straight lines that have slight curvature or jitter.
- **The "Speckle"**: Tiny islands of color that should be merged into the background.
- **The "Gap"**: White spaces or cracks between adjacent shapes.
- **The "Blob"**: Sharp details (like serifs on text) smoothed out into blobs.
- **The "Wrong Color"**: Gradients approximated by banding or wrong palette selection.

### 3. Decide (Targeted Optimization)
Choose a strategy to fix the identified error.

**Strategy A: Palette Optimization (Critical for Logos)**
Logos usually have a limited, discrete palette.
- *Action*: Use the `--colors` argument to force a specific palette size (e.g., 2, 4, 8).
- *Action*: Tune K-means clustering parameters to better separate similar shades.

**Strategy B: Geometric Tuning**
- *Action*: Adjust `corner_threshold` to preserve sharpness.
- *Action*: Increase `path_precision` to capture subtle curves in illustrations.
- *Action*: Use `--quality ultra` for maximum vertex count optimization.

**Strategy C: Topology Repair**
- *Action*: If holes are filled (e.g., inside an 'O'), adjust the `layer_difference` threshold.

### 4. Act (Implementation & Verification)
Implement the fix and verify.

**Step 1: Single Asset Verification**
Test ONLY the problematic asset.
```bash
# Use the specific logo command for best results
vectalab logo test_data/png_multi/brand.png --quality ultra --colors 4
```

**Step 2: Regression Test**
Ensure no other logos degraded.
```bash
python scripts/quick_baseline.py
```

## Success Criteria & Metrics (SOTA Standards)

| Category | Metric | SOTA Target | Min Acceptable | Description |
|----------|--------|-------------|----------------|-------------|
| **Structural** | **SSIM** | **> 99.95%** | 99.5% | Indistinguishable to the eye. |
| **Perceptual** | **Visual Fidelity** | **> 99.99%** | 99.8% | Structural similarity ignoring noise. |
| **Geometric** | **Topology** | **100%** | 99.0% | Perfect preservation of holes/islands. |
| **Geometric** | **Edge Accuracy** | **> 99.0%** | 98.0% | Edges align within sub-pixel precision. |
| **Color** | **Color Error (ΔE)** | **< 1.0** | < 2.3 | Imperceptible color difference. |
| **Efficiency** | **Path Count** | **< 1.2x** | < 2x | Clean, human-readable SVG code. |
| **Performance** | **Time** | **< 2s/logo** | < 10s/logo | Fast enough for real-time workflows. |

### Metric Definitions

- **SSIM**: Standard structural similarity. For logos, we demand near-perfection.
- **Visual Fidelity**: Perceptual SSIM. Critical for illustrations where minor noise is acceptable but shape distortion is not.
- **Topology Score**: The ratio of matched connected components and holes. 100% means the vector has the exact same structure as the raster.
- **Edge Accuracy**: Canny edge detection overlap. Ensures lines are where they should be.
- **Color Error (ΔE)**: CIEDE2000. SOTA requires $\Delta E < 1.0$ (visually identical).

## Troubleshooting

- **Text looks bad?** Text is the hardest part of logo vectorization. Ensure `corner_threshold` is high (60 degrees+) to keep serifs sharp.
- **Too many colors?** The algorithm might be detecting compression artifacts as new colors. Use `--colors <N>` to force reduction.
- **Gaps between shapes?** This is a common issue with "tracing" algorithms. Ensure "layer stacking" is enabled to place shapes on top of a background rather than adjacent to it.

## Future Improvements
- **OCR Integration**: Detect text and replace with actual fonts.
- **Gradient Mesh**: Support true SVG gradients instead of solid bands.
- **Symmetry Detection**: Enforce perfect symmetry for geometric logos.
