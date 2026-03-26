# CLI

## Convert

```bash
cargo run --release -- convert INPUT.png [OUTPUT.svg] \
  --target 0.92 \
  --max-file-size 100000 \
  --max-iterations 4 \
  --quality balanced \
  --method hifi
```

Notes:

- `quality` can be `figma`, `balanced`, `quality`, or `ultra`
- `method` can be `hifi`, `logo`, `premium`, `auto`, `smart`, `optimal`, `bayesian`, or `sam`
- native `smart`, `optimal`, and `bayesian` routes perform real strategy selection in Rust
- `sam` remains a compatibility route and currently falls back to the native auto strategy
- if no output is provided, the tool writes `INPUT.svg`
- `--json` prints the full conversion report

## Smart

```bash
cargo run --release -- smart INPUT.png [OUTPUT.svg] \
  --quality 0.92 \
  --size 100 \
  --iterations 5
```

Runs the iterative native strategy search modeled after Python `vectalab.sota.vectorize_smart`.

## Info

```bash
cargo run --release -- info INPUT.png
```

Prints file metadata, image analysis, and the recommended conversion strategy.

## Analyze

```bash
cargo run --release -- analyze INPUT.png
```

Prints the detected image type, complexity, color count, and edge density.

## Compare

```bash
cargo run --release -- compare INPUT.png OUTPUT.svg
```

Prints `ssim`, `psnr`, `mae`, file size, and path count.

## Render

```bash
cargo run --release -- render OUTPUT.svg PREVIEW.png
```

Renders SVG output back to PNG using the built-in Rust renderer.

## Optimize

```bash
cargo run --release -- optimize INPUT.svg [OUTPUT.svg] --precision 2
```

Optimizes an existing SVG in place by default using the built-in Rust optimizer.

## Benchmark

```bash
cargo run --release -- benchmark \
  --input-dir examples \
  --output-dir benchmark_runs/latest \
  --markdown-path benchmark_runs/latest/report.md \
  --json-path benchmark_runs/latest/report.json
```

## Golden Benchmark

```bash
cargo run --release -- benchmark-golden \
  --golden-dir golden_data \
  --work-dir benchmark_runs/golden_full \
  --json-path benchmark_runs/golden_full/report.json \
  --markdown-path benchmark_runs/golden_full/report.md
```
