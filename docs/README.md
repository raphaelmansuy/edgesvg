# EdgeSVG Documentation

This set is organized around delivery surfaces, not around source files.

## Start Here

- [cli.md](cli.md): installation, command map, copy-pasteable examples
- [api.md](api.md): Rust crate and stable SDK-facing request/response contract
- [python_sdk.md](python_sdk.md): Python install, API, return shapes
- [node_sdk.md](node_sdk.md): Node install, build flow, package behavior
- [wasm_sdk.md](wasm_sdk.md): browser and bundler-facing API
- [algorithm.md](algorithm.md): how EdgeSVG chooses and scores candidates
- [benchmarks.md](benchmarks.md): reproducible quality regression workflow
- [ci_and_publishing.md](ci_and_publishing.md): CI matrix and release automation

## Reading Order

1. Read the root `README.md` for the overall product surface.
2. Use [cli.md](cli.md) if you need a binary workflow.
3. Use [api.md](api.md) plus the SDK page for your target language.
4. Use [benchmarks.md](benchmarks.md) and [algorithm.md](algorithm.md) when changing vectorization behavior.
