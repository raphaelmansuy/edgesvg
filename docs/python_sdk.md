# Python SDK

## Local Install

```bash
cd sdks/python
maturin develop
```

## Package API

```python
import edgesvg

result = edgesvg.vectorize("input.png", method="auto")
analysis = edgesvg.analyze("input.png")
info = edgesvg.inspect("input.png")
metrics = edgesvg.compare("input.png", result["svg"])
optimized = edgesvg.optimize_svg(result["svg"])
preview = edgesvg.render_png(result["svg"], 512, 512)
```

## Functions

| Function | Return |
|---|---|
| `vectorize(...)` | dict shaped like `VectorizeResponse` |
| `analyze(path)` | dict with `analysis` and `decision` |
| `inspect(path)` | dict with recommendation data |
| `compare(path, svg)` | `QualityMetrics` dict |
| `optimize_svg(svg, precision=2)` | optimization summary dict |
| `render_png(svg, width, height)` | `bytes` |
| `benchmark(...)` | `BenchmarkReport` dict |
| `benchmark_golden(...)` | `BenchmarkReport` dict |
| `version()` | version string |

## CLI Wrapper

The Python package also exposes a simple `edgesvg` console entrypoint through `edgesvg.cli`.
