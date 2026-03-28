/**
 * Web Worker — runs edgesvg-wasm off the main thread.
 *
 * Messages:
 *   → { type: 'vectorize', id, bytes, request }
 *   → { type: 'analyze', id, bytes }
 *   ← { type: 'ready' }
 *   ← { type: 'result', id, ok, data?, error? }
 */

import wasmInit, { vectorize, analyze, version } from '../../wasm-pkg/edgesvg_wasm.js';

let ready = false;

async function init() {
  await wasmInit();
  ready = true;
  self.postMessage({ type: 'ready', version: version() });
}

self.addEventListener('message', (e: MessageEvent) => {
  const msg = e.data;

  if (msg.type === 'vectorize') {
    if (!ready) {
      self.postMessage({ type: 'result', id: msg.id, ok: false, error: 'WASM not ready' });
      return;
    }
    try {
      const bytes = new Uint8Array(msg.bytes);
      const result = vectorize(bytes, msg.request ?? undefined);
      self.postMessage({ type: 'result', id: msg.id, ok: true, data: result });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      self.postMessage({ type: 'result', id: msg.id, ok: false, error: message });
    }
  }

  if (msg.type === 'analyze') {
    if (!ready) {
      self.postMessage({ type: 'result', id: msg.id, ok: false, error: 'WASM not ready' });
      return;
    }
    try {
      const bytes = new Uint8Array(msg.bytes);
      const result = analyze(bytes);
      self.postMessage({ type: 'result', id: msg.id, ok: true, data: result });
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      self.postMessage({ type: 'result', id: msg.id, ok: false, error: message });
    }
  }
});

// Pre-warm WASM on worker creation
init();
