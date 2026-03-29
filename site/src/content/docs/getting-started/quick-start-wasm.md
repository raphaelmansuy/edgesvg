---
title: WebAssembly Quick Start
description: Run EdgeSVG in the browser with WebAssembly. Full Rust engine client-side — no server, no uploads, privacy by design.
sidebar:
  order: 5
---

## What is the EdgeSVG WASM SDK?

The WebAssembly SDK compiles the full EdgeSVG Rust engine to a browser-native WASM binary. You get:

- **Same engine** as the CLI/Python/Node.js SDKs
- **No server required** — all processing runs in the user's browser
- **Privacy by design** — image data never leaves the device
- **Works offline** after the first WASM load (~4 MB cached)
- **Zero infrastructure cost** — deploy as static files

## Install

```bash
npm install edgesvg-wasm
# or use the pre-built pkg/ from the repo
```

## Quick Start

```typescript
import init, { vectorize, analyze, compare, optimize_svg, render } from 'edgesvg-wasm';

// Load WASM binary — call this once on app startup
await init();

// Get bytes from a file input or fetch
const file = inputElement.files[0];
const bytes = new Uint8Array(await file.arrayBuffer());

// Vectorize the image
const result = vectorize(bytes, { method: 'auto' });

console.log('SVG:', result.svg.slice(0, 100));
console.log('SSIM:', result.report.metrics.ssim.toFixed(4));
console.log('Method:', result.effectiveMethod);
```

## React/Vue Integration

### React example

```tsx
import { useState, useRef } from 'react';
import init, { vectorize } from 'edgesvg-wasm';

// Initialize once on app load
await init();

export function VectorizerDemo() {
  const [svg, setSvg] = useState<string | null>(null);
  const [metrics, setMetrics] = useState<any>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const handleFile = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    const bytes = new Uint8Array(await file.arrayBuffer());
    const result = vectorize(bytes, { method: 'auto' });

    setSvg(result.svg);
    setMetrics(result.report.metrics);
  };

  return (
    <div>
      <input type="file" accept="image/*" onChange={handleFile} ref={inputRef} />
      {svg && (
        <div>
          <div dangerouslySetInnerHTML={{ __html: svg }} />
          <p>SSIM: {metrics.ssim.toFixed(4)}</p>
          <p>Edge F1: {metrics.edge_f1.toFixed(4)}</p>
        </div>
      )}
    </div>
  );
}
```

## Full Browser API

```typescript
import init, {
  vectorize,
  analyze,
  compare,
  optimize_svg,
  render,
} from 'edgesvg-wasm';

await init();

const bytes = new Uint8Array(await file.arrayBuffer());

// Vectorize
const result = vectorize(bytes, { method: 'auto' });
// result.svg — SVG string
// result.report.metrics — quality metrics
// result.effectiveMethod — method that was used
// result.decision — explanation

// Analyze image type
const analysis = analyze(bytes);
// analysis.analysis.kind — "logo" | "icon" | "illustration" | "photo"
// analysis.decision — explanation string

// Compare raster vs. SVG
const metrics = compare(bytes, result.svg);
// metrics.ssim, metrics.edge_f1, metrics.fidelity_score, ...

// Optimize SVG
const optimized = optimize_svg(result.svg, 2);  // precision = 2

// Render SVG to PNG
const pngBytes = render(result.svg, 512, 512);
const blob = new Blob([pngBytes], { type: 'image/png' });
const url = URL.createObjectURL(blob);
```

## Vite Configuration

```typescript
// vite.config.ts
import { defineConfig } from 'vite';

export default defineConfig({
  server: {
    headers: {
      // Required for SharedArrayBuffer / WASM threading
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
    },
  },
  // Tell Vite not to transform the WASM file
  assetsInclude: ['**/*.wasm'],
});
```

## Build from Source

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build the WASM package
wasm-pack build crates/edgesvg-wasm --target bundler --out-dir pkg --release

# Output: crates/edgesvg-wasm/pkg/
```

For browser use without a bundler:

```bash
wasm-pack build crates/edgesvg-wasm --target web --out-dir pkg --release
```

## Live Demo

Try the EdgeSVG WASM SDK live: [**Open Demo →**](/guides/demo/)

Drag and drop any PNG, JPG, or WebP image. Vectorization runs entirely in your browser — no uploads, no server.
