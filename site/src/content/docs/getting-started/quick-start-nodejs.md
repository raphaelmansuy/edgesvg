---
title: Node.js Quick Start
description: Get started with EdgeSVG in Node.js. Convert PNG, JPG, and WebP to scored SVG output with the native NAPI-RS addon.
sidebar:
  order: 2
---

## Install

```bash
npm install edgesvg
# or
yarn add edgesvg
# or
pnpm add edgesvg
```

Node.js 18+. Pre-built native addons for macOS (arm64, x64), Linux (x64, arm64), and Windows (x64).

## Basic Usage

### Vectorize an image

```typescript
import { vectorize } from 'edgesvg';

// Auto mode — EdgeSVG picks the best strategy
const result = vectorize('logo.png', { method: 'auto' });

console.log('SVG length:', result.svg.length);
console.log('Method used:', result.effectiveMethod);
console.log('Decision:', result.decision);
```

### Inspect quality metrics

```typescript
const { metrics } = result.report;
console.log('SSIM:       ', metrics.ssim.toFixed(4));
console.log('Edge F1:    ', metrics.edgeF1.toFixed(4));
console.log('Foreground: ', metrics.foregroundIou.toFixed(4));
console.log('Path count: ', metrics.pathCount);
console.log('File size:  ', metrics.fileSize, 'bytes');
```

### Analyze before converting

```typescript
import { analyze, inspect } from 'edgesvg';

// Get recommendation
const info = inspect('logo.png');
console.log('Recommendation:', info.recommendation);

// Raw image features
const analysis = analyze('logo.png');
console.log('Image kind:', analysis.analysis.kind);
console.log('Decision:  ', analysis.decision);
```

### Compare SVG quality

```typescript
import { compare } from 'edgesvg';

const metrics = compare('logo.png', result.svg);
console.log('SSIM:     ', metrics.ssim.toFixed(4));
console.log('Fidelity: ', metrics.fidelityScore.toFixed(4));
```

## Vectorization Modes

| Method | When to use |
|--------|-------------|
| `'auto'` | Delegate choice to EdgeSVG (recommended) |
| `'logo'` | Logos, icons, geometric shapes |
| `'premium'` | Higher-fidelity richer images |
| `'smart'` | Iterative search to hit quality targets |

```typescript
const logo   = vectorize('icon.png',         { method: 'logo' });
const premium = vectorize('illustration.png', { method: 'premium' });
const smart  = vectorize('complex.png',       { method: 'smart' });
```

## Full API

```typescript
import {
  vectorize,
  analyze,
  inspect,
  compare,
  optimizeSvg,
  renderPng,
  benchmark,
  version,
} from 'edgesvg';

// Vectorize
const result = vectorize('input.png', { method: 'auto' });

// Inspect / analyze
const info     = inspect('input.png');
const analysis = analyze('input.png');

// Compare SVG with raster
const metrics = compare('input.png', svgString);

// Optimize SVG (coordinate rounding, noise removal)
const optimized = optimizeSvg(svgString, { precision: 2 });

// Render SVG to PNG
const pngBuffer = renderPng(svgString, 512, 512);

// Batch benchmark
const report = benchmark({ inputDirs: ['images/'], outputDir: 'results/' });

// Version
console.log(version());
```

## Save and Load

```typescript
import { vectorize, renderPng } from 'edgesvg';
import { writeFileSync } from 'fs';

const result = vectorize('logo.png', { method: 'auto' });

// Save SVG
writeFileSync('logo.svg', result.svg, 'utf-8');

// Render and save PNG preview
const png = renderPng(result.svg, 512, 512);
writeFileSync('preview.png', Buffer.from(png));
```

## TypeScript Support

The package includes TypeScript type declarations. All API functions are fully typed.

```typescript
import type { VectorizeResult, QualityMetrics, AnalyzeResult } from 'edgesvg';
```

## CLI via npx

```bash
npx edgesvg convert logo.png logo.svg --method auto --json
```

## Build from source

```bash
git clone https://github.com/raphaelmansuy/edgesvg.git
cd edgesvg/sdks/node
npm ci
npm run build
```

Requires Rust 1.82+ and Node.js 18+.
