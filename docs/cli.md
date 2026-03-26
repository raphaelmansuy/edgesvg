# CLI

## Convert

```bash
cargo run --release -- convert INPUT.png [OUTPUT.svg] \
  --target-ssim 0.92 \
  --max-file-size 100000 \
  --max-iterations 4 \
  --quality balanced
```

Notes:

- `quality` can be `compact`, `balanced`, `quality`, or `ultra`
- if no output is provided, the tool writes `INPUT.svg`
- `--json` prints the full conversion report

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

## Benchmark

```bash
cargo run --release -- benchmark \
  --input-dir examples \
  --output-dir benchmark_runs/latest \
  --markdown-path benchmark_runs/latest/report.md \
  --json-path benchmark_runs/latest/report.json
```
