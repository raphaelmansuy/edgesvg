import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const repoRoot = path.resolve(root, '..', '..');

const platformMap = {
  'linux-x64': {
    targetFile: 'libedgesvg_node.so',
    packageDir: 'linux-x64-gnu',
    outputFile: 'edgesvg-node.linux-x64-gnu.node'
  },
  'linux-arm64': {
    targetFile: 'libedgesvg_node.so',
    packageDir: 'linux-arm64-gnu',
    outputFile: 'edgesvg-node.linux-arm64-gnu.node'
  },
  'darwin-x64': {
    targetFile: 'libedgesvg_node.dylib',
    packageDir: 'darwin-x64',
    outputFile: 'edgesvg-node.darwin-x64.node'
  },
  'darwin-arm64': {
    targetFile: 'libedgesvg_node.dylib',
    packageDir: 'darwin-arm64',
    outputFile: 'edgesvg-node.darwin-arm64.node'
  },
  'win32-x64': {
    targetFile: 'edgesvg_node.dll',
    packageDir: 'win32-x64-msvc',
    outputFile: 'edgesvg-node.win32-x64-msvc.node'
  }
};

const key = `${process.platform}-${process.arch}`;
const platform = platformMap[key];

if (!platform) {
  throw new Error(`Unsupported platform: ${key}`);
}

const source = path.join(repoRoot, 'target', 'release', platform.targetFile);
const nativeOut = path.join(root, 'native', 'edgesvg.node');
const packageOut = path.join(root, 'npm', platform.packageDir, platform.outputFile);

fs.mkdirSync(path.dirname(nativeOut), { recursive: true });
fs.mkdirSync(path.dirname(packageOut), { recursive: true });
fs.copyFileSync(source, nativeOut);
fs.copyFileSync(source, packageOut);
