# CLI

## Install

### From source

```bash
cargo install --path .
```

### Local development

```bash
cargo run -- convert examples/test_logo_benchmark.png out.svg
```

## Command Model

| Command | When to use it |
|---|---|
| `convert` | Default single-image conversion path |
| `logo` | Logos and icons where path cleanliness matters most |
| `premium` | Higher-fidelity path for richer images |
| `auto` | Delegate method choice to EdgeSVG |
| `smart` | Iterative search under fidelity and size constraints |
| `info` | Quick recommendation for an input |
| `analyze` | Raw image features and classification |
| `compare` | Quantify how good an SVG is |
| `render` | Produce a PNG preview from SVG |
| `optimize` | Round coordinates and remove SVG noise |
| `benchmark` | Batch convert a raster directory |
| `benchmark-golden` | Re-render the golden SVG corpus into rasters and compare |

## Examples

### Default conversion

```bash
edgesvg convert input.png output.svg
```

### Auto mode with machine-readable output

```bash
edgesvg convert input.png output.svg --method auto --json
```

### Premium conversion with explicit target

```bash
edgesvg premium input.png output.svg --target 0.98 --json
```

### Inspect an input before conversion

```bash
edgesvg info input.png --json
edgesvg analyze input.png --json
```

### Compare SVG quality

```bash
edgesvg compare input.png output.svg --json
```

### Render SVG preview

```bash
edgesvg render output.svg preview.png
```

### Optimize an existing SVG

```bash
edgesvg optimize output.svg --precision 2 --json
```

### Batch benchmark

```bash
edgesvg benchmark \
  --input-dir examples \
  --output-dir benchmark_runs/latest \
  --json-path benchmark_runs/latest/report.json \
  --markdown-path benchmark_runs/latest/report.md
```

## JSON Contract

The CLI emits the same report objects that the SDKs use:

- vectorization returns `report.metrics`, `requested_method`, and `effective_method`
- `info` returns recommendation fields
- `compare` returns the full `QualityMetrics` object
- benchmark commands return a `BenchmarkReport`

If you are scripting against EdgeSVG, prefer `--json` everywhere.
