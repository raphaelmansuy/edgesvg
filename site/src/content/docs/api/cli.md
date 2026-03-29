---
title: CLI Reference
description: Complete reference for the EdgeSVG command-line interface.
---

The EdgeSVG CLI is a single binary that can be installed via Cargo.

```bash
cargo install edgesvg
```

## Global Options

```
edgesvg [OPTIONS] <COMMAND>
```

| Option | Short | Description |
|--------|-------|-------------|
| `--help` | `-h` | Print help |
| `--version` | `-V` | Print version |
| `--verbose` | `-v` | Verbose output |

## `convert`

Convert a single image to SVG.

```bash
edgesvg convert [OPTIONS] <INPUT> [OUTPUT]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Path to input image (PNG, JPG, WebP, GIF) |
| `[OUTPUT]` | Output SVG path (defaults to `<INPUT>.svg`) |

### Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--method <METHOD>` | `-m` | `auto` | Vectorization method: `auto`, `logo`, `premium`, `smart`, `convert` |
| `--target-ssim <FLOAT>` | | `0.995` | Target SSIM for `smart` method |
| `--max-file-size <BYTES>` | | none | Max output file size |
| `--json` | `-j` | false | Print quality report as JSON |
| `--verbose` | `-v` | false | Verbose output |

### Examples

```bash
# Basic conversion
edgesvg convert logo.png logo.svg

# Auto mode with JSON report
edgesvg convert input.png output.svg --method auto --json

# Target quality with smart mode
edgesvg convert input.png output.svg --method smart --target-ssim 0.998

# Premium mode, verbose
edgesvg convert illustration.png --method premium --verbose
```

### Example JSON output

```json
{
  "svg": "...",
  "requested_method": "Auto",
  "effective_method": "Logo",
  "fallback_from": null,
  "decision": "Image classified as logo (low unique colors, high edge density)",
  "report": {
    "score": 0.984,
    "grade": "A",
    "summary": "Excellent vectorization quality",
    "metrics": {
      "ssim": 0.9980,
      "edge_f1": 0.9870,
      "path_count": 4,
      "file_size": 1842
    }
  }
}
```

---

## `batch`

Convert multiple images with glob patterns.

```bash
edgesvg batch [OPTIONS] <INPUT_GLOB> [OUTPUT_DIR]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--method` | `auto` | Vectorization method for all inputs |
| `--jobs` | CPU count | Parallel worker threads |
| `--json` | false | Write `report.json` to output dir |
| `--target-ssim` | `0.995` | Quality target for `smart` method |
| `--continue-on-error` | false | Skip failing items instead of stopping |

### Examples

```bash
# Batch convert folder
edgesvg batch "assets/icons/*.png" assets/icons/svg/

# Parallel with 8 threads
edgesvg batch "logos/**/*.png" out/ --jobs 8

# Write report
edgesvg batch "*.png" svg/ --json
```

---

## `benchmark`

Run quality benchmarks against a set of golden images.

```bash
edgesvg benchmark [OPTIONS] <GOLDEN_DIR>
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--output` | stdout | Output path for results CSV/JSON |
| `--format` | `table` | `table`, `json`, `csv` |

### Example

```bash
edgesvg benchmark golden_data/ --format json --output benchmark_results.json
```

---

## `inspect`

Print quality metrics for an existing SVG against its source image.

```bash
edgesvg inspect <ORIGINAL_IMAGE> <SVG_FILE>
```

### Example

```bash
edgesvg inspect logo.png logo.svg
# SSIM: 0.9980  EdgeF1: 0.9870  Grade: A
```

---

## `version`

Print the installed version.

```bash
edgesvg version
# edgesvg 0.1.0 (Rust 1.82, built 2025-01-01)
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Conversion failed |
| `2` | Quality threshold not met (smart mode) |
| `3` | Invalid arguments |
| `4` | I/O error |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `EDGESVG_LOG` | Logging level: `error`, `warn`, `info`, `debug`, `trace` |
| `EDGESVG_JOBS` | Override default parallelism for batch commands |
| `EDGESVG_TARGET_SSIM` | Override default SSIM target |
