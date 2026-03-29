---
title: WebAssembly SDK Reference
description: Complete API reference for the EdgeSVG WebAssembly SDK for browser and edge environments.
---

The EdgeSVG WASM SDK runs entirely in the browser — no server required. Built with `wasm-pack`, it exposes the same interface as the other SDKs.

## Installation

### npm / pnpm

```bash
npm install edgesvg-wasm
# or
pnpm add edgesvg-wasm
```

### CDN (browser, no bundler)

```html
<script type="module">
  import init, { vectorize } from 'https://cdn.jsdelivr.net/npm/edgesvg-wasm/edgesvg_wasm.js';
  await init();
  // ready to use
</script>
```

## Initialization

You must call `init()` once before any other function. This loads the `.wasm` binary.

```typescript
import init, { vectorize, version } from 'edgesvg-wasm';

await init();
console.log(version());
```

## `vectorize()`

```typescript
function vectorize(imageData: Uint8Array, options?: WasmVectorizeOptions): WasmVectorizeResponse
```

### Options

```typescript
interface WasmVectorizeOptions {
  method?: 'auto' | 'logo' | 'premium' | 'smart' | 'convert';
  targetSsim?: number;
  maxFileSize?: number;
}
```

### Response

```typescript
interface WasmVectorizeResponse {
  svg: string;
  requestedMethod: string;
  effectiveMethod: string;
  fallbackFrom?: string;
  decision?: string;
  score: number;
  grade: string;
  metrics: {
    ssim: number;
    ssimPerceptual: number;
    edgeSimilarity: number;
    edgePrecision: number;
    edgeRecall: number;
    edgeF1: number;
    foregroundIou: number;
    colorSimilarity: number;
    fidelityScore: number;
    deltaE: number;
    topologyScore: number;
    psnr: number;
    mae: number;
    fileSize: number;
    pathCount: number;
    weightedPathCount: number;
  };
}
```

## Examples

### File input in browser

```html
<input type="file" id="file-input" accept="image/*" />
<div id="output"></div>

<script type="module">
  import init, { vectorize } from 'edgesvg-wasm';
  await init();

  document.getElementById('file-input').addEventListener('change', async (e) => {
    const file = e.target.files[0];
    if (!file) return;

    const arrayBuf = await file.arrayBuffer();
    const bytes = new Uint8Array(arrayBuf);

    const result = vectorize(bytes, { method: 'auto' });

    document.getElementById('output').innerHTML = result.svg;
    console.log(`Grade: ${result.grade}, SSIM: ${result.metrics.ssim.toFixed(4)}`);
  });
</script>
```

### Vue / React component

```typescript
import init, { vectorize } from 'edgesvg-wasm';

let initialized = false;

async function ensureInit() {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

export async function convertImage(file: File): Promise<{ svg: string; grade: string }> {
  await ensureInit();
  const bytes = new Uint8Array(await file.arrayBuffer());
  const result = vectorize(bytes, { method: 'auto' });
  return { svg: result.svg, grade: result.grade };
}
```

### Web Worker (non-blocking)

Run vectorization in a worker to keep the UI thread responsive:

```typescript
// worker.ts
import init, { vectorize } from 'edgesvg-wasm';

self.addEventListener('message', async ({ data }) => {
  if (data.type === 'init') {
    await init();
    self.postMessage({ type: 'ready' });
  }
  if (data.type === 'vectorize') {
    const result = vectorize(data.bytes, data.options);
    self.postMessage({ type: 'result', result });
  }
});

// main.ts
const worker = new Worker(new URL('./worker.ts', import.meta.url), { type: 'module' });
worker.postMessage({ type: 'init' });
worker.addEventListener('message', ({ data }) => {
  if (data.type === 'result') {
    renderSvg(data.result.svg);
  }
});
```

## Bundler Configuration

### Vite

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';

export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
});
```

### Webpack 5

```javascript
// webpack.config.js
module.exports = {
  experiments: {
    asyncWebAssembly: true,
  },
};
```

### Next.js

```javascript
// next.config.js
module.exports = {
  webpack(config) {
    config.experiments = { ...config.experiments, asyncWebAssembly: true };
    return config;
  },
};
```

## Performance Notes

- The `.wasm` binary is ~2–4 MB; cache it via a service worker for repeat visits
- `vectorize()` is synchronous but CPU-bound — use a Web Worker for images > 500 KB
- The WASM build uses SIMD where available for a 2–4× speedup in supported browsers
