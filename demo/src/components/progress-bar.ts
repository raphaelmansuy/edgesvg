/** Progress bar — thin animated bar shown during vectorization. */

import { el } from '../utils/dom';
import { store } from '../state';

export function createProgressBar(): HTMLElement {
  const bar = el('div', { className: 'progress-bar' });
  const indicator = el('div', { className: 'progress-bar__indicator' });
  bar.appendChild(indicator);

  store.subscribe('processStatus', (val) => {
    const status = val as string;
    bar.classList.toggle('progress-bar--active', status === 'vectorizing' || status === 'analyzing');
  });

  return bar;
}
