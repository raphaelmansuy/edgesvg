/** Resizable split pane. */

import { el } from '../utils/dom';

export function createSplitPane(left: HTMLElement, right: HTMLElement): HTMLElement {
  const leftPane = el('div', { className: 'split-pane__left' });
  leftPane.appendChild(left);

  const rightPane = el('div', { className: 'split-pane__right' });
  rightPane.appendChild(right);

  const handle = el('div', {
    className: 'split-pane__handle',
    role: 'separator',
    ariaLabel: 'Resize panes',
    tabindex: '0',
  });

  const wrapper = el('div', { className: 'split-pane' });
  wrapper.append(leftPane, handle, rightPane);

  // Drag to resize
  let dragging = false;

  handle.addEventListener('mousedown', (e: Event) => {
    e.preventDefault();
    dragging = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  });

  document.addEventListener('mousemove', (e: MouseEvent) => {
    if (!dragging) return;
    const rect = wrapper.getBoundingClientRect();
    const pct = ((e.clientX - rect.left) / rect.width) * 100;
    const clamped = Math.min(Math.max(pct, 20), 80);
    leftPane.style.width = `${clamped}%`;
    rightPane.style.width = `${100 - clamped}%`;
  });

  document.addEventListener('mouseup', () => {
    if (!dragging) return;
    dragging = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  });

  return wrapper;
}
