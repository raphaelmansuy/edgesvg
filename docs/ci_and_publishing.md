# CI And Publishing

Vectalab is structured as a standard publishable Rust crate.

## Local Verification

Run the same checks you should expect in CI:

```bash
cargo fmt --check
cargo test
```

For release validation:

```bash
cargo build --release
cargo run --release -- benchmark --input-dir examples --output-dir benchmark_runs/latest
```

## Publishing To crates.io

Dry run first:

```bash
cargo publish --dry-run
```

Publish:

```bash
cargo publish
```

## Release Discipline

- bump `version` in `Cargo.toml`
- keep `Cargo.lock` committed
- run tests before tagging
- include benchmark notes when vectorization logic changes
- tag releases consistently, for example `v0.2.1`
