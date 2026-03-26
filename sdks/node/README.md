# EdgeSVG Node.js SDK

`edgesvg` exposes the Rust vectorizer to Node.js through a native N-API addon.

## Install

```bash
npm install edgesvg
```

## Example

```ts
import { vectorize, renderPng } from 'edgesvg';

const result = vectorize('logo.png', { method: 'auto' });
console.log(result.report.metrics.ssim);

const preview = renderPng(result.svg, 512, 512);
```
