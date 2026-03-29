---
title: CLI Quick Start
description: EdgeSVG command-line reference. Convert, analyze, compare, and batch-process raster images to SVG from the terminal.
sidebar:
  order: 3
---

## Install

### From source (Rust 1.82+)

```bash
cargo install --path .
```

### Local development

```bash
cargo run -- convert examples/test_logo_benchmark.png out.svg
```

## Commands

| Command | Purpose |
|---------|---------|
| `convert` | Default single-image conversion |
| `logo` | Logo and icon oriented conversion (clean paths) |
| `premium` | Higher-fidelity conversion for richer images |
| `auto` | Let EdgeSVG choose the method |
| `smart` | Iterative search under quality and size constraints |
| `info` | Quick recommendation for an input |
| `analyze` | Raw image features and classification |
| `compare` | Quantify SVG quality against source raster |
| `render` | Produce a PNG preview from an SVG |
| `optimize` | Round coordinates and remove SVG noise |
| `benchmark` | Batch convert a raster directory |
| `benchmark-golden` | Re-render golden SVG corpus and compare |

## Examples

### Convert with auto mode

```bash
edgesvg convert logo.png logo.svg --method auto --json
```

The `--json` flag outputs a machine-readable JSON report with quality metrics:

```json
{
  "svg": "...",
  "requested_method": "Auto",
  "effective_method": "Logo",
  "fallback_from": null,
  "decision": "Detected as logo (monochrome, low unique colors)",
  "report": {
    "metrics": {
      "ssim": 0.9987,
      "edge_f1": 0.9423,
      "foreground_iou": 0.9812,
      "path_count": 3,
      "file_size": 1248
    }
  }
}
```

### Logo conversion

```bash
edgesvg logo icon.png icon.svg --json
```

### Premium high-fidelity

```bash
edgesvg premium illustration.png illustration.svg --target 0.98 --json
```

### Inspect an image

```bash
# Quick recommendation
edgesvg info logo.png --json

# Raw image features
edgesvg analyze logo.png --json
```

### Compare SVG quality

```bash
edgesvg compare logo.png logo.svg --json
```

### Render SVG to PNG

```bash
edgesvg render logo.svg preview.png --width 512 --height 512
```

### Optimize an SVG

```bash
edgesvg optimize logo.svg --precision 2 --json
```

### Batch benchmark

```bash
edgesvg benchmark \
  --input-dir images/ \
  --output-dir results/ \
  --json-path results/report.json \
  --markdown-path results/report.md
```

### Golden benchmark

Re-render golden SVG corpus into rasters and compare against the current engine:

```bash
edgesvg benchmark-golden --json
```

## JSON Output

All commands support `--json` for machine-readable output. This is the recommended format for scripting:

```bash
# Pipe to jq for quick metric inspection
edgesvg convert logo.png logo.svg --method auto --json | jq '.report.metrics.ssim'

# Check if quality meets threshold
SSIM=$(edgesvg compare logo.png logo.svg --json | jq '.ssim')
if (( $(echo "$SSIM < 0.99" | bc -l) )); then
  echo "Quality regression detected"
  exit 1
fi
```

## CI/CD Integration

Add quality gates to your build pipeline:

```yaml
# .github/workflows/vectorize.yml
- name: Vectorize assets
  run: |
    edgesvg convert assets/logo.png dist/logo.svg --method auto --json > report.json
    SSIM=$(jq '.report.metrics.ssim' report.json)
    python3 -c "import sys; sys.exit(0 if float('$SSIM') >= 0.98 else 1)"
```

See the [CI/CD guide](/guides/cicd/) for a complete example.
