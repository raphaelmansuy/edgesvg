---
title: Python SDK Reference
description: Complete API reference for the EdgeSVG Python SDK.
---

The Python SDK wraps the Rust core via PyO3/maturin. Install with pip:

```bash
pip install edgesvg
```

## `vectorize()`

The primary entry point for all vectorization operations.

```python
import edgesvg

result = edgesvg.vectorize(
    input_path: str,
    *,
    output_path: str | None = None,
    method: str = "auto",
    target_ssim: float | None = None,
    max_file_size: int | None = None,
    verbose: bool = False,
) -> dict
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `input_path` | `str` | required | Path to input image (PNG, JPG, WebP, GIF) |
| `output_path` | `str \| None` | `None` | Save SVG here; if None returns inline |
| `method` | `str` | `"auto"` | `"auto"`, `"logo"`, `"premium"`, `"smart"`, `"convert"` |
| `target_ssim` | `float \| None` | `None` | Target SSIM for `smart` mode |
| `max_file_size` | `int \| None` | `None` | Max bytes for output SVG |
| `verbose` | `bool` | `False` | Enable verbose output |

### Return value

A `dict` with the following keys:

```python
{
    "svg": "<svg>...</svg>",           # Generated SVG string
    "requested_method": "Auto",        # Method you asked for
    "effective_method": "Logo",        # Method that ran
    "fallback_from": None,             # Original method if fell back
    "decision": "Classified as logo",  # Explanation (auto mode)
    "report": {
        "score": 0.982,
        "grade": "A",
        "summary": "...",
        "metrics": {
            "ssim": 0.9980,
            "ssim_perceptual": 0.9990,
            "edge_similarity": 0.9920,
            "edge_precision": 0.9880,
            "edge_recall": 0.9861,
            "edge_f1": 0.9870,
            "foreground_iou": 0.9790,
            "color_similarity": 0.9960,
            "fidelity_score": 0.9950,
            "delta_e": 1.234,
            "topology_score": 0.9020,
            "psnr": 42.3,
            "mae": 0.008,
            "file_size": 4096,
            "path_count": 5,
            "weighted_path_count": 3.2,
        }
    }
}
```

## `vectorize_bytes()`

Convert image data from memory (no disk I/O).

```python
import edgesvg

with open("input.png", "rb") as f:
    image_data = f.read()

result = edgesvg.vectorize_bytes(
    data: bytes,
    *,
    method: str = "auto",
    target_ssim: float | None = None,
    max_file_size: int | None = None,
) -> dict
```

Useful for web servers, batch processing pipelines, and environments where file paths are unavailable.

## `version()`

```python
edgesvg.version() -> str
# "0.1.0"
```

## Full Examples

### Basic conversion

```python
import edgesvg

result = edgesvg.vectorize("logo.png", output_path="logo.svg")
print(f"Grade: {result['report']['grade']}")
print(f"SSIM: {result['report']['metrics']['ssim']:.4f}")
```

### Quality-gated conversion

```python
import edgesvg

result = edgesvg.vectorize(
    "input.png",
    method="smart",
    target_ssim=0.995,
)

metrics = result["report"]["metrics"]
assert metrics["ssim"] >= 0.99, f"SSIM too low: {metrics['ssim']}"
print("Quality check passed!")
```

### Batch conversion

```python
import edgesvg
from pathlib import Path
import json

def batch_convert(input_dir: str, output_dir: str):
    Path(output_dir).mkdir(parents=True, exist_ok=True)
    results = []

    for png in Path(input_dir).glob("*.png"):
        out = Path(output_dir) / png.with_suffix(".svg").name
        result = edgesvg.vectorize(str(png), output_path=str(out))
        results.append({
            "file": png.name,
            "grade": result["report"]["grade"],
            "ssim": result["report"]["metrics"]["ssim"],
        })

    with open(f"{output_dir}/report.json", "w") as f:
        json.dump(results, f, indent=2)

    return results
```

### Flask endpoint

```python
from flask import Flask, request, jsonify
import edgesvg

app = Flask(__name__)

@app.post("/vectorize")
def vectorize():
    data = request.get_data()
    if not data:
        return jsonify({"error": "no image data"}), 400

    result = edgesvg.vectorize_bytes(data, method="auto")
    return jsonify({
        "svg": result["svg"],
        "grade": result["report"]["grade"],
        "ssim": result["report"]["metrics"]["ssim"],
    })
```

## Type Stubs

The package ships with `edgesvg.pyi` stub files for full editor support with mypy and Pyright.

```python
from edgesvg import vectorize, VectorizeResponse  # typed

result: VectorizeResponse = vectorize("input.png")
```
