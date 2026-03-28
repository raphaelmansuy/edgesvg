/** Parameters panel — vectorization settings. */

import { el } from '../utils/dom';
import { store } from '../state';
import type { QualityPreset, VectorizeMethod } from '../types';

const METHODS: { value: VectorizeMethod; label: string }[] = [
  { value: 'auto', label: 'Auto' },
  { value: 'hifi', label: 'Hi-Fi' },
  { value: 'logo', label: 'Logo' },
  { value: 'premium', label: 'Premium' },
  { value: 'smart', label: 'Smart' },
  { value: 'optimal', label: 'Optimal' },
  { value: 'bayesian', label: 'Bayesian' },
];

const QUALITIES: { value: QualityPreset; label: string }[] = [
  { value: 'figma', label: 'Figma' },
  { value: 'balanced', label: 'Balanced' },
  { value: 'quality', label: 'Quality' },
  { value: 'ultra', label: 'Ultra' },
];

export function createParamsPanel(): HTMLElement {
  const panel = el('div', { className: 'params-panel' });

  // Scrollable content area
  const content = el('div', { className: 'params-panel__content' });
  const title = el('div', { className: 'params-panel__title', textContent: 'Settings' });

  // Method selector
  const methodGroup = el('div', { className: 'params-panel__group' });
  const methodLabel = el('label', { className: 'params-panel__label', textContent: 'Method' });
  const methodSelect = el('select', { className: 'params-panel__select' }) as HTMLSelectElement;
  for (const m of METHODS) {
    const opt = el('option', { value: m.value, textContent: m.label });
    if (m.value === store.get('method')) (opt as HTMLOptionElement).selected = true;
    methodSelect.appendChild(opt);
  }
  methodSelect.addEventListener('change', () => {
    store.set('method', methodSelect.value as VectorizeMethod);
  });
  methodGroup.append(methodLabel, methodSelect);

  // Quality chips
  const qualityGroup = el('div', { className: 'params-panel__group' });
  const qualityLabel = el('label', { className: 'params-panel__label', textContent: 'Quality' });
  const chips = el('div', { className: 'params-panel__chips' });
  const chipButtons: HTMLButtonElement[] = [];

  for (const q of QUALITIES) {
    const chip = el('button', {
      className: `params-panel__chip${q.value === store.get('quality') ? ' params-panel__chip--active' : ''}`,
      textContent: q.label,
    }) as HTMLButtonElement;
    chip.dataset.value = q.value;
    chip.addEventListener('click', () => {
      store.set('quality', q.value);
      chipButtons.forEach((b) =>
        b.classList.toggle('params-panel__chip--active', b.dataset.value === q.value),
      );
    });
    chipButtons.push(chip);
    chips.appendChild(chip);
  }
  qualityGroup.append(qualityLabel, chips);

  // ── Advanced params inside a <details> disclosure ──
  const advanced = el('details', { className: 'params-panel__advanced' }) as HTMLDetailsElement;
  const summary = el('summary', { textContent: 'Advanced' });
  const advancedBody = el('div', { className: 'params-panel__advanced-body' });

  // Target SSIM slider
  const ssimGroup = el('div', { className: 'params-panel__group' });
  const ssimLabel = el('label', { className: 'params-panel__label', textContent: 'Target SSIM' });
  const ssimWrap = el('div', { className: 'params-panel__range-wrap' });
  const ssimRange = el('input', {
    type: 'range',
    className: 'params-panel__range',
    min: '0.90',
    max: '1.0',
    step: '0.001',
    value: String(store.get('targetSsim')),
    title: 'Structural Similarity — higher = more faithful to original',
  }) as HTMLInputElement;
  const ssimValue = el('span', {
    className: 'params-panel__range-value',
    textContent: store.get('targetSsim').toFixed(3),
  });
  ssimRange.addEventListener('input', () => {
    const v = parseFloat(ssimRange.value);
    store.set('targetSsim', v);
    ssimValue.textContent = v.toFixed(3);
  });
  ssimWrap.append(ssimRange, ssimValue);
  ssimGroup.append(ssimLabel, ssimWrap);

  // Max iterations slider
  const iterGroup = el('div', { className: 'params-panel__group' });
  const iterLabel = el('label', { className: 'params-panel__label', textContent: 'Iterations' });
  const iterWrap = el('div', { className: 'params-panel__range-wrap' });
  const iterRange = el('input', {
    type: 'range',
    className: 'params-panel__range',
    min: '1',
    max: '10',
    step: '1',
    value: String(store.get('maxIterations')),
  }) as HTMLInputElement;
  const iterValue = el('span', {
    className: 'params-panel__range-value',
    textContent: String(store.get('maxIterations')),
  });
  iterRange.addEventListener('input', () => {
    const v = parseInt(iterRange.value, 10);
    store.set('maxIterations', v);
    iterValue.textContent = String(v);
  });
  iterWrap.append(iterRange, iterValue);
  iterGroup.append(iterLabel, iterWrap);

  // Max file size slider
  const sizeGroup = el('div', { className: 'params-panel__group' });
  const sizeLabel = el('label', { className: 'params-panel__label', textContent: 'Max Size' });
  const sizeWrap = el('div', { className: 'params-panel__range-wrap' });
  const sizeRange = el('input', {
    type: 'range',
    className: 'params-panel__range',
    min: '10000',
    max: '500000',
    step: '10000',
    value: String(store.get('maxFileSize')),
  }) as HTMLInputElement;
  const sizeValue = el('span', {
    className: 'params-panel__range-value',
    textContent: formatSize(store.get('maxFileSize')),
  });
  sizeRange.addEventListener('input', () => {
    const v = parseInt(sizeRange.value, 10);
    store.set('maxFileSize', v);
    sizeValue.textContent = formatSize(v);
  });
  sizeWrap.append(sizeRange, sizeValue);
  sizeGroup.append(sizeLabel, sizeWrap);

  // Colors input
  const colorsGroup = el('div', { className: 'params-panel__group' });
  const colorsLabel = el('label', { className: 'params-panel__label', textContent: 'Colors' });
  const colorsInput = el('input', {
    type: 'number',
    className: 'params-panel__input',
    placeholder: 'Auto',
    min: '2',
    max: '256',
  }) as HTMLInputElement;
  colorsInput.addEventListener('change', () => {
    const v = colorsInput.value ? parseInt(colorsInput.value, 10) : null;
    store.set('colors', v);
  });
  colorsGroup.append(colorsLabel, colorsInput);

  advancedBody.append(ssimGroup, iterGroup, sizeGroup, colorsGroup);
  advanced.append(summary, advancedBody);

  content.append(title, methodGroup, qualityGroup, advanced);
  panel.appendChild(content);

  // Sticky footer with Vectorize button
  const footer = el('div', { className: 'params-panel__footer' });
  const vectorizeBtn = el('button', {
    className: 'params-panel__btn',
    disabled: true,
  }) as HTMLButtonElement;
  vectorizeBtn.innerHTML = `<svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M2.5 1.5L10 6L2.5 10.5V1.5Z" fill="currentColor"/></svg> Vectorize`;

  vectorizeBtn.addEventListener('click', () => {
    store.dispatchEvent(new CustomEvent('vectorize-requested'));
  });

  // Add keyboard shortcut: Cmd/Ctrl + Enter
  document.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      if (!vectorizeBtn.disabled) store.dispatchEvent(new CustomEvent('vectorize-requested'));
    }
  });

  store.subscribe('processStatus', (val) => {
    const status = val as string;
    vectorizeBtn.disabled = status === 'vectorizing';
    if (status === 'vectorizing') {
      vectorizeBtn.innerHTML = `<svg width="12" height="12" viewBox="0 0 12 12" fill="currentColor" xmlns="http://www.w3.org/2000/svg" style="animation:es-spin .7s linear infinite"><path d="M6 1a5 5 0 100 10A5 5 0 006 1zm0 1.5a3.5 3.5 0 010 7 3.5 3.5 0 010-7z" opacity=".25"/><path d="M6 1a5 5 0 014.33 2.5" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round"/></svg> Vectorizing\u2026`;
    } else {
      vectorizeBtn.innerHTML = `<svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M2.5 1.5L10 6L2.5 10.5V1.5Z" fill="currentColor"/></svg> Vectorize`;
    }
  });

  store.subscribe('imageBytes', (val) => {
    const hasImage = !!val;
    const isVectorizing = store.get('processStatus') === 'vectorizing';
    vectorizeBtn.disabled = !hasImage || isVectorizing;
  });

  footer.appendChild(vectorizeBtn);
  panel.appendChild(footer);

  return panel;
}

function formatSize(bytes: number): string {
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)}M`;
  return `${Math.round(bytes / 1000)}K`;
}
