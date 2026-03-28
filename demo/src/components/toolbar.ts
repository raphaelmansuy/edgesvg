/** Toolbar: brand, upload button, dark mode toggle, WASM status. */

import { el } from '../utils/dom';
import { store } from '../state';
import { getVersion } from '../services/wasm-bridge';
import { handleFile } from './drop-zone';

const ACCEPTED_EXT = '.png,.jpg,.jpeg,.webp,.gif,.bmp';

const SVG_MOON = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"/></svg>`;
const SVG_SUN  = `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`;

export function createToolbar(): HTMLElement {
  const header = el('header', { className: 'toolbar' });

  // Brand
  const brand = el('div', {
    className: 'toolbar__brand',
    innerHTML: `<span class="toolbar__logo">ES</span><span class="toolbar__title">EdgeSVG</span>`,
  });

  // WASM status pill
  const wasmPill = el('span', {
    className: 'toolbar__wasm-status toolbar__wasm-status--loading',
    textContent: 'Loading WASM...',
  });

  store.subscribe('wasmStatus', (val) => {
    if (val === 'ready') {
      wasmPill.textContent = `WASM v${getVersion()}`;
      wasmPill.className = 'toolbar__wasm-status toolbar__wasm-status--ready';
      setTimeout(() => { (wasmPill as HTMLElement).style.opacity = '0.5'; }, 4000);
    } else if (val === 'error') {
      wasmPill.textContent = 'WASM error';
      wasmPill.className = 'toolbar__wasm-status';
      (wasmPill as HTMLElement).style.color = 'var(--es-error)';
    }
  });

  // Upload button
  const fileInput = el('input', {
    id: 'image-upload-input',
    type: 'file',
    accept: ACCEPTED_EXT,
    className: 'toolbar__file-input',
  }) as HTMLInputElement;

  const uploadBtn = el(
    'label',
    {
      className: 'toolbar__btn toolbar__btn--upload',
      ariaLabel: 'Upload an image file',
      role: 'button',
      tabindex: '0',
    },
    'Upload Image',
    fileInput,
  ) as HTMLLabelElement;

  uploadBtn.addEventListener('keydown', (e: Event) => {
    const ke = e as KeyboardEvent;
    if (ke.key === 'Enter' || ke.key === ' ') {
      ke.preventDefault();
      fileInput.click();
    }
  });

  fileInput.addEventListener('change', () => {
    const file = fileInput.files?.[0];
    if (file) handleFile(file);
    fileInput.value = '';
  });

  // Dark mode toggle (SVG icon)
  const darkBtn = el('button', {
    className: 'toolbar__btn toolbar__btn--icon',
    innerHTML: SVG_MOON,
    ariaLabel: 'Toggle dark mode',
    title: 'Toggle dark mode',
  }) as HTMLButtonElement;

  darkBtn.addEventListener('click', () => {
    const next = !store.get('darkMode');
    store.set('darkMode', next);
    document.documentElement.classList.toggle('dark', next);
    darkBtn.innerHTML = next ? SVG_SUN : SVG_MOON;
  });

  const divider = el('span', { className: 'toolbar__divider', ariaHidden: 'true' });

  const actions = el('div', { className: 'toolbar__actions' });
  actions.append(uploadBtn, divider, darkBtn);

  header.append(brand, wasmPill, actions);
  return header;
}
