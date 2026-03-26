# Demo

Quick local demo:

```bash
cargo run --release -- convert examples/test_logo_benchmark.png demo/output.svg
cargo run --release -- render demo/output.svg demo/output.png
cargo run --release -- compare examples/test_logo_benchmark.png demo/output.svg
```

This produces:

- `demo/output.svg`
- `demo/output.png`

Use the benchmark command if you want a folder-level report instead of a single-file demo.
