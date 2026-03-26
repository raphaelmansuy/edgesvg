import { createRequire } from 'node:module';
import path from 'node:path';

const require = createRequire(import.meta.url);

type NativeModule = {
  vectorizeJson: (inputPath: string, options?: Record<string, unknown>) => string;
  analyzeJson: (inputPath: string) => string;
  inspectJson: (inputPath: string) => string;
  compareJson: (inputPath: string, svg: string) => string;
  optimizeJson: (svg: string, precision?: number) => string;
  renderPngBuffer: (svg: string, width: number, height: number) => Buffer;
  benchmarkJson: (inputDir: string, outputDir: string, options?: Record<string, unknown>) => string;
  benchmarkGoldenJson: (goldenDir: string, workDir: string, options?: Record<string, unknown>) => string;
  version: () => string;
};

function packageName(): string {
  const platforms: Record<string, string> = {
    'linux-x64': 'edgesvg-linux-x64-gnu',
    'linux-arm64': 'edgesvg-linux-arm64-gnu',
    'darwin-x64': 'edgesvg-darwin-x64',
    'darwin-arm64': 'edgesvg-darwin-arm64',
    'win32-x64': 'edgesvg-win32-x64-msvc'
  };
  const key = `${process.platform}-${process.arch}`;
  const pkg = platforms[key];
  if (!pkg) {
    throw new Error(`edgesvg: unsupported platform: ${key}`);
  }
  return pkg;
}

function localBinaryPath(): string {
  return path.join(path.dirname(new URL(import.meta.url).pathname), '..', 'native', 'edgesvg.node');
}

export function loadNative(): NativeModule {
  try {
    return require(packageName());
  } catch {
    return require(localBinaryPath());
  }
}
