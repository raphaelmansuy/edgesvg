# Node.js SDK

## Local Install

```bash
cd sdks/node
npm ci
npm run build
```

## Package API

```ts
import { analyze, compare, optimizeSvg, renderPng, vectorize } from 'edgesvg';

const result = vectorize('input.png', { method: 'auto' });
const analysis = analyze('input.png');
const metrics = compare('input.png', result.svg);
const optimized = optimizeSvg(result.svg);
const preview = renderPng(result.svg, 512, 512);
```

## Functions

| Function | Return |
|---|---|
| `vectorize(path, options?)` | `VectorizeResponse` |
| `analyze(path)` | plain object |
| `inspect(path)` | plain object |
| `compare(path, svg)` | `QualityMetrics` |
| `optimizeSvg(svg, precision?)` | plain object |
| `renderPng(svg, width, height)` | `Buffer` |
| `benchmark(...)` | plain object |
| `benchmarkGolden(...)` | plain object |
| `version()` | version string |

## Packaging Model

The published `edgesvg` package is a lightweight wrapper that resolves a platform-specific optional dependency containing the compiled `.node` binary. Local development falls back to `sdks/node/native/edgesvg.node`.
