# EdgeSVG Python SDK

`edgesvg` exposes the native Rust vectorizer to Python with zero shelling out.

## Install

```bash
pip install edgesvg
```

## Example

```python
import edgesvg

result = edgesvg.vectorize("logo.png", method="auto")
print(result["report"]["metrics"]["ssim"])

png_preview = edgesvg.render_png(result["svg"], 512, 512)
```
