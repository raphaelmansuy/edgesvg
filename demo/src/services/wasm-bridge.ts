/**
 * WASM bridge — manages the Web Worker and exposes async API.
 */

import type { AnalyzeResponse, VectorizeRequest, VectorizeResponse } from '../types';
import { store } from '../state';

type WorkerResponse =
  | { type: 'ready'; version: string }
  | { type: 'result'; id: string; ok: true; data: unknown }
  | { type: 'result'; id: string; ok: false; error: string };

const worker = new Worker(
  new URL('../workers/vectorize-worker.ts', import.meta.url),
  { type: 'module' },
);

const pending = new Map<
  string,
  { resolve: (v: unknown) => void; reject: (e: unknown) => void }
>();

let wasmVersion = '';

worker.addEventListener('message', (e: MessageEvent<WorkerResponse>) => {
  const msg = e.data;

  if (msg.type === 'ready') {
    wasmVersion = msg.version;
    store.set('wasmStatus', 'ready');
    return;
  }

  if (msg.type === 'result') {
    const p = pending.get(msg.id);
    if (!p) return;
    pending.delete(msg.id);

    if (msg.ok) {
      p.resolve(msg.data);
    } else {
      p.reject(new Error(msg.error));
    }
  }
});

worker.addEventListener('error', (e) => {
  store.set('wasmStatus', 'error');
  store.set('errorMessage', `Worker error: ${e.message}`);
});

store.set('wasmStatus', 'loading');

let nextId = 0;

export function getVersion(): string {
  return wasmVersion;
}

export function vectorizeImage(
  bytes: Uint8Array,
  request: VectorizeRequest,
): Promise<VectorizeResponse> {
  const id = String(nextId++);
  const promise = new Promise<VectorizeResponse>((resolve, reject) => {
    pending.set(id, {
      resolve: resolve as (v: unknown) => void,
      reject,
    });
  });
  worker.postMessage({ type: 'vectorize', id, bytes: bytes.buffer, request });
  return promise;
}

export function analyzeImage(bytes: Uint8Array): Promise<AnalyzeResponse> {
  const id = String(nextId++);
  const promise = new Promise<AnalyzeResponse>((resolve, reject) => {
    pending.set(id, {
      resolve: resolve as (v: unknown) => void,
      reject,
    });
  });
  worker.postMessage({ type: 'analyze', id, bytes: bytes.buffer });
  return promise;
}
