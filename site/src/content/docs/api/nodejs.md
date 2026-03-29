---
title: Node.js SDK Reference
description: Complete API reference for the EdgeSVG Node.js / TypeScript SDK.
---

The Node.js SDK wraps the Rust core via NAPI-RS, providing full TypeScript types.

```bash
npm install edgesvg-node
# or
pnpm add edgesvg-node
```

## `vectorize()`

```typescript
import { vectorize } from 'edgesvg-node';

const result = await vectorize(inputPath: string, options?: VectorizeOptions): Promise<VectorizeResponse>
```

### Options

```typescript
interface VectorizeOptions {
  outputPath?: string;
  method?: 'auto' | 'logo' | 'premium' | 'smart' | 'convert';
  targetSsim?: number;
  maxFileSize?: number;
  verbose?: boolean;
}
```

### Response

```typescript
interface VectorizeResponse {
  svg: string;
  requestedMethod: string;
  effectiveMethod: string;
  fallbackFrom?: string;
  decision?: string;
  report: QualityReport;
}

interface QualityReport {
  score: number;        // 0.0 – 1.0
  grade: string;        // "A+", "A", "B" …
  summary: string;
  metrics: QualityMetrics;
}

interface QualityMetrics {
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
}
```

## `vectorizeBuffer()`

Convert from a `Buffer` or `Uint8Array` in memory.

```typescript
import { vectorizeBuffer } from 'edgesvg-node';
import { readFileSync } from 'fs';

const buf = readFileSync('input.png');
const result = await vectorizeBuffer(buf, { method: 'logo' });
console.log(result.svg);
```

## `version()`

```typescript
import { version } from 'edgesvg-node';
console.log(version()); // "0.1.0"
```

## Examples

### Basic conversion

```typescript
import { vectorize } from 'edgesvg-node';

const result = await vectorize('logo.png', { outputPath: 'logo.svg' });
console.log(`Grade: ${result.report.grade}`);
console.log(`SSIM: ${result.report.metrics.ssim.toFixed(4)}`);
```

### Parallel batch processing

```typescript
import { vectorize } from 'edgesvg-node';
import { glob } from 'glob';
import path from 'path';

async function batchConvert(inputDir: string, outputDir: string) {
  const files = await glob(`${inputDir}/*.png`);

  const results = await Promise.all(
    files.map(async (file) => {
      const name = path.basename(file, '.png');
      const output = path.join(outputDir, `${name}.svg`);
      const result = await vectorize(file, { output, method: 'auto' });
      return { file: name, grade: result.report.grade, ssim: result.report.metrics.ssim };
    })
  );

  return results;
}
```

### Express endpoint

```typescript
import express from 'express';
import { vectorizeBuffer } from 'edgesvg-node';
import multer from 'multer';

const app = express();
const upload = multer({ storage: multer.memoryStorage() });

app.post('/vectorize', upload.single('image'), async (req, res) => {
  if (!req.file) return res.status(400).json({ error: 'No image uploaded' });

  try {
    const result = await vectorizeBuffer(req.file.buffer, { method: 'auto' });
    res.json({
      svg: result.svg,
      grade: result.report.grade,
      ssim: result.report.metrics.ssim,
    });
  } catch (err) {
    res.status(500).json({ error: 'Vectorization failed' });
  }
});
```

### CI quality check

```typescript
import { vectorize } from 'edgesvg-node';

const result = await vectorize('logo.png', { method: 'premium' });
const { ssim, edgeF1 } = result.report.metrics;

if (ssim < 0.99 || edgeF1 < 0.95) {
  console.error(`Quality check failed: SSIM=${ssim.toFixed(4)} EdgeF1=${edgeF1.toFixed(4)}`);
  process.exit(1);
}

console.log(`✓ Quality passed: grade ${result.report.grade}`);
```

## CommonJS Support

The package ships both ESM and CJS builds:

```javascript
const { vectorize } = require('edgesvg-node');
```
