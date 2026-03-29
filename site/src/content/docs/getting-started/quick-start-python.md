---
title: Python Quick Start
description: Get started with EdgeSVG in Python in under a minute. Convert PNG, JPG, and WebP to SVG with full quality scoring.
sidebar:
  order: 1
---

## Install

```bash
pip install edgesvg
```

Python 3.9+. Pre-built wheels for macOS (arm64, x64), Linux (x64, arm64), and Windows (x64).

## Basic Usage

### Vectorize an image

```python
import edgesvg

# Auto mode — EdgeSVG picks the best strategy for the image
result = edgesvg.vectorize("logo.png", method="auto")

# The result contains svg, report, and metadata
print(result["svg"][:200])           # SVG string
print(result["requested_method"])    # what you asked for
print(result["effective_method"])    # what was actually used
print(result["decision"])            # why
```

### Inspect quality metrics

```python
metrics = result["report"]["metrics"]
print(f"SSIM:        {metrics['ssim']:.4f}")
print(f"Edge F1:     {metrics['edge_f1']:.4f}")
print(f"Foreground:  {metrics['foreground_iou']:.4f}")
print(f"Path count:  {metrics['path_count']}")
print(f"File size:   {metrics['file_size']} bytes")
```

### Analyze an image before converting

```python
# Get a recommendation and image classification
info = edgesvg.inspect("logo.png")
print(info["recommendation"])   # "logo", "icon", "illustration", "photo"

# Get raw image features
analysis = edgesvg.analyze("logo.png")
print(analysis["analysis"]["kind"])        # image kind
print(analysis["analysis"]["edge_density"])
print(analysis["decision"])
```

### Compare an SVG against the source

```python
# Compute quality metrics between raster and SVG
metrics = edgesvg.compare("logo.png", result["svg"])
print(f"SSIM:          {metrics['ssim']:.4f}")
print(f"Fidelity:      {metrics['fidelity_score']:.4f}")
print(f"Color sim:     {metrics['color_similarity']:.4f}")
print(f"Topology:      {metrics['topology_score']:.4f}")
```

## Vectorization Methods

| Method | When to use |
|--------|-------------|
| `auto` | Delegate method choice to EdgeSVG (recommended for most cases) |
| `logo` | Logos, icons, and geometric shapes — clean minimal paths |
| `premium` | Higher-fidelity conversion for richer images |
| `smart` | Iterative search under SSIM and file-size constraints |

```python
# Logo mode — clean paths, minimal complexity
result = edgesvg.vectorize("icon.png", method="logo")

# Premium mode — higher fidelity
result = edgesvg.vectorize("illustration.png", method="premium")

# Smart mode — hit a target SSIM
result = edgesvg.vectorize("complex.png", method="smart")
```

## Full API

```python
import edgesvg

# Vectorize (main entrypoint)
result = edgesvg.vectorize(
    path="input.png",          # path to input image
    method="auto",             # method: auto, logo, premium, smart
)

# Inspect / analyze
info     = edgesvg.inspect("input.png")
analysis = edgesvg.analyze("input.png")

# Compare existing SVG with raster source
metrics = edgesvg.compare("input.png", svg_string)

# Optimize an SVG (coordinate rounding, noise removal)
optimized = edgesvg.optimize_svg(svg_string, precision=2)

# Render SVG to PNG bytes
png_bytes = edgesvg.render_png(svg_string, width=512, height=512)

# Batch benchmark a directory
report = edgesvg.benchmark(
    input_dirs=["images/"],
    output_dir="results/",
)

# SDK version
print(edgesvg.version())
```

## Save and Load

```python
import edgesvg

result = edgesvg.vectorize("logo.png", method="auto")

# Save SVG
with open("logo.svg", "w", encoding="utf-8") as f:
    f.write(result["svg"])

# Render a PNG preview
png_bytes = edgesvg.render_png(result["svg"], 512, 512)
with open("preview.png", "wb") as f:
    f.write(png_bytes)
```

## CLI entry point

The Python package also exposes an `edgesvg` console command:

```bash
edgesvg convert logo.png logo.svg --method auto --json
```

## Build from source

```bash
git clone https://github.com/raphaelmansuy/edgesvg.git
cd edgesvg
cd sdks/python
pip install maturin
maturin develop --release
```

Requires Rust 1.82+.
