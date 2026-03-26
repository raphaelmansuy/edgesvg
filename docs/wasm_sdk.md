# WASM SDK

## Build

```bash
wasm-pack build crates/edgesvg-wasm --target bundler --out-dir pkg
```

## Browser API

```ts
import init, { analyze, compare, optimize_svg, render, vectorize } from './pkg/edgesvg_wasm.js';

await init();

const result = vectorize(fileBytes, { method: 'auto' });
const analysis = analyze(fileBytes);
const metrics = compare(fileBytes, result.svg);
const optimized = optimize_svg(result.svg, 2);
const previewPng = render(result.svg, 512, 512);
```

## Contract

The WASM wrapper uses the same `VectorizeRequest` object as the Rust SDK layer and returns the same serializable response shapes.

## Current Constraint

`wasm-pack build` is the full package-generation step. `cargo check -p edgesvg-wasm --target wasm32-unknown-unknown` is the lighter CI validation step.
