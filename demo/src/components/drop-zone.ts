/** Drop zone + image preview + parameters panel (left pane). */

import { el } from '../utils/dom';
import { store } from '../state';

const ACCEPTED_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif', 'image/bmp'];
const ACCEPTED_EXT = '.png,.jpg,.jpeg,.webp,.gif,.bmp';

export function createDropZone(): HTMLElement {
  const wrapper = el('div', { className: 'left-pane' });

  // Pane header with label + file info meta
  const paneHeader = el('div', { className: 'pane-header' });
  const paneLabel = el('span', { className: 'pane-header__label', textContent: 'Source' });
  const paneMeta = el('span', { className: 'pane-header__meta' });
  paneHeader.append(paneLabel, paneMeta);

  // Update file info in header when image loads
  store.subscribe('fileName', (val) => {
    paneMeta.textContent = val as string;
  });
  store.subscribe('imageBytes', (val) => {
    const bytes = val as Uint8Array | null;
    if (bytes) {
      const size = bytes.length < 1024 ? `${bytes.length}B`
        : bytes.length < 1_048_576 ? `${(bytes.length/1024).toFixed(0)}KB`
        : `${(bytes.length/1_048_576).toFixed(1)}MB`;
      paneMeta.title = `${store.get('fileName')} — ${size}`;
    } else {
      paneMeta.textContent = '';
      paneMeta.title = '';
    }
  });

  // Drop zone (empty state)
  const dropZone = el('div', { className: 'drop-zone' });
  const inner = el('div', { className: 'drop-zone__inner' });

  const icon = el('div', {
    className: 'drop-zone__icon',
    innerHTML: `<svg viewBox="0 0 48 48" fill="none" xmlns="http://www.w3.org/2000/svg">
      <rect x="6" y="10" width="36" height="28" rx="4" stroke="currentColor" stroke-width="2.5"/>
      <circle cx="18" cy="22" r="4" stroke="currentColor" stroke-width="2"/>
      <path d="M6 34l10-10 8 8 6-6 12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>`,
  });

  const title = el('div', { className: 'drop-zone__title', textContent: 'Drop an image here' });
  const sub = el('div', { className: 'drop-zone__sub', textContent: 'or click to browse' });
  const hint = el('div', { className: 'drop-zone__hint', textContent: 'PNG, JPG, WebP, GIF, BMP' });

  inner.append(icon, title, sub, hint);
  dropZone.appendChild(inner);

  // Hidden file input
  const fileInput = el('input', {
    type: 'file',
    accept: ACCEPTED_EXT,
    className: 'toolbar__file-input',
  }) as HTMLInputElement;
  dropZone.appendChild(fileInput);

  inner.addEventListener('click', () => fileInput.click());
  fileInput.addEventListener('change', () => {
    const file = fileInput.files?.[0];
    if (file) handleFile(file);
    fileInput.value = '';
  });

  // Image preview (shown when image is loaded)
  const preview = el('div', { className: 'image-preview' });
  const img = el('img', { className: 'image-preview__img' }) as HTMLImageElement;
  img.alt = 'Source image';
  preview.appendChild(img);
  preview.style.display = 'none';

  wrapper.append(paneHeader, dropZone, preview);

  // React to state changes
  store.subscribe('imageUrl', (val) => {
    const url = val as string | null;
    if (url) {
      img.src = url;
      dropZone.style.display = 'none';
      preview.style.display = 'flex';
    } else {
      dropZone.style.display = 'flex';
      preview.style.display = 'none';
    }
  });

  return wrapper;
}

export function handleFile(file: File): void {
  if (!ACCEPTED_TYPES.includes(file.type)) {
    store.set('errorMessage', `Unsupported file type: ${file.type}`);
    return;
  }

  const reader = new FileReader();
  reader.onload = () => {
    const buffer = reader.result as ArrayBuffer;
    const bytes = new Uint8Array(buffer);

    // Revoke previous URL
    const prev = store.get('imageUrl');
    if (prev) URL.revokeObjectURL(prev);

    const url = URL.createObjectURL(new Blob([bytes], { type: file.type }));
    store.set('fileName', file.name);
    store.set('imageBytes', bytes);
    store.set('imageUrl', url);
    store.set('result', null);
    store.set('processStatus', 'idle');
  };
  reader.onerror = () => {
    store.set('errorMessage', `Failed to read file: ${reader.error?.message}`);
  };
  reader.readAsArrayBuffer(file);
}

export function enableDragDrop(): void {
  const body = document.body;
  let dragCounter = 0;

  body.addEventListener('dragover', (e) => {
    e.preventDefault();
  });

  body.addEventListener('dragenter', (e) => {
    e.preventDefault();
    dragCounter++;
    body.classList.add('drag-over');
  });

  body.addEventListener('dragleave', () => {
    dragCounter--;
    if (dragCounter <= 0) {
      dragCounter = 0;
      body.classList.remove('drag-over');
    }
  });

  body.addEventListener('drop', (e) => {
    e.preventDefault();
    dragCounter = 0;
    body.classList.remove('drag-over');
    const file = e.dataTransfer?.files[0];
    if (file) handleFile(file);
  });
}
