import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { describe, expect, test } from 'vitest';
import { analyze, compare, optimizeSvg, renderPng, vectorize } from '../src/index.js';

const here = path.dirname(new URL(import.meta.url).pathname);

describe('edgesvg', () => {
  test('vectorize and compare fixture', () => {
    const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'edgesvg-node-'));
    const input = path.join(tempDir, 'fixture.png');
    const fixture = path.resolve(here, '../../../examples/test_logo_benchmark.png');
    fs.copyFileSync(fixture, input);

    const result = vectorize(input, { method: 'auto' });
    expect(result.svg).toContain('<svg');

    const analysis = analyze(input);
    expect(analysis).toHaveProperty('analysis.width');

    const metrics = compare(input, result.svg);
    expect(metrics.ssim).toBeGreaterThanOrEqual(0);

    const optimized = optimizeSvg(result.svg);
    expect(String(optimized.optimized_svg)).toContain('<svg');

    const png = renderPng(result.svg, 64, 32);
    expect(png.subarray(0, 4).toString('hex')).toBe('89504e47');
  });
});
