import { loadNative } from './native.js';
import type { BenchmarkOptions, QualityMetrics, VectorizeOptions, VectorizeResponse } from './types.js';

let native: ReturnType<typeof loadNative> | undefined;

function getNative() {
  if (!native) {
    native = loadNative();
  }
  return native;
}

function parseJson<T>(payload: string): T {
  return JSON.parse(payload) as T;
}

export function vectorize(inputPath: string, options?: VectorizeOptions): VectorizeResponse {
  return parseJson<VectorizeResponse>(getNative().vectorizeJson(inputPath, options ? {
    method: options.method,
    target_ssim: options.targetSsim,
    max_file_size: options.maxFileSize,
    max_iterations: options.maxIterations,
    quality: options.quality,
    logo_quality: options.logoQuality,
    colors: options.colors
  } : undefined));
}

export function analyze(inputPath: string): Record<string, unknown> {
  return parseJson<Record<string, unknown>>(getNative().analyzeJson(inputPath));
}

export function inspect(inputPath: string): Record<string, unknown> {
  return parseJson<Record<string, unknown>>(getNative().inspectJson(inputPath));
}

export function compare(inputPath: string, svg: string): QualityMetrics {
  return parseJson<QualityMetrics>(getNative().compareJson(inputPath, svg));
}

export function optimizeSvg(svg: string, precision = 2): Record<string, unknown> {
  return parseJson<Record<string, unknown>>(getNative().optimizeJson(svg, precision));
}

export function renderPng(svg: string, width: number, height: number): Buffer {
  return getNative().renderPngBuffer(svg, width, height);
}

export function benchmark(
  inputDir: string,
  outputDir: string,
  options?: BenchmarkOptions
): Record<string, unknown> {
  return parseJson<Record<string, unknown>>(getNative().benchmarkJson(inputDir, outputDir, options ? {
    target_ssim: options.targetSsim,
    max_file_size: options.maxFileSize,
    max_iterations: options.maxIterations,
    quality: options.quality,
    limit: options.limit
  } : undefined));
}

export function benchmarkGolden(
  goldenDir: string,
  workDir: string,
  options?: BenchmarkOptions
): Record<string, unknown> {
  return parseJson<Record<string, unknown>>(getNative().benchmarkGoldenJson(goldenDir, workDir, options ? {
    target_ssim: options.targetSsim,
    max_file_size: options.maxFileSize,
    max_iterations: options.maxIterations,
    quality: options.quality,
    limit: options.limit
  } : undefined));
}

export function version(): string {
  return getNative().version();
}

export type { BenchmarkOptions, QualityMetrics, VectorizeOptions, VectorizeResponse };
