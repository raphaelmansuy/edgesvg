import './style.css';
import { mountApp } from './components/app-shell';
import { store } from './state';
import { vectorizeImage } from './services/wasm-bridge';
import type { VectorizeRequest, VectorizeResponse } from './types';

// Mount the application shell
const root = document.querySelector<HTMLDivElement>('#app');
if (!root) throw new Error('Missing #app element');
mountApp(root);

// Handle vectorize requests from the params panel button
store.addEventListener('vectorize-requested', async () => {
  const bytes = store.get('imageBytes');
  if (!bytes) return;

  const wasmStatus = store.get('wasmStatus');
  if (wasmStatus !== 'ready') {
    store.set('errorMessage', 'WASM is still loading — please wait');
    return;
  }

  store.set('processStatus', 'vectorizing');
  store.set('errorMessage', null);

  const request: VectorizeRequest = {
    method: store.get('method'),
    quality: store.get('quality'),
    target_ssim: store.get('targetSsim'),
    max_file_size: store.get('maxFileSize'),
    max_iterations: store.get('maxIterations'),
    colors: store.get('colors') ?? undefined,
  };

  try {
    const result: VectorizeResponse = await vectorizeImage(bytes, request);
    store.set('result', result);
    store.set('processStatus', 'done');
    store.set('activeTab', 'svg');
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    store.set('errorMessage', `Vectorization failed: ${message}`);
    store.set('processStatus', 'error');
  }
});
