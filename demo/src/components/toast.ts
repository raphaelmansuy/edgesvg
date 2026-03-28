/** Toast notification system. */

import { el } from '../utils/dom';
import { store } from '../state';

let container: HTMLElement | null = null;

function ensureContainer(): HTMLElement {
  if (!container) {
    container = el('div', { className: 'toast-container' });
    document.body.appendChild(container);
  }
  return container;
}

function showToast(message: string, type: 'error' | 'success' | 'info' = 'info') {
  const c = ensureContainer();

  const closeBtn = el('button', {
    className: 'toast__close',
    textContent: '\u00d7',
    ariaLabel: 'Dismiss',
  });

  const toast = el(
    'div',
    { className: `toast toast--${type}` },
    el('span', {}, message),
    closeBtn,
  );

  closeBtn.addEventListener('click', () => toast.remove());
  c.appendChild(toast);

  setTimeout(() => toast.remove(), 5000);
}

export function initToastListener(): void {
  store.subscribe('errorMessage', (val) => {
    const msg = val as string | null;
    if (msg) showToast(msg, 'error');
  });
}

export { showToast };
