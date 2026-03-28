/** Output panel — tabs (SVG | Code | Info), download/copy actions. */

import { el } from '../utils/dom';
import { store } from '../state';
import type { OutputTab, VectorizeResponse } from '../types';
import { showToast } from './toast';

export function createOutputPanel(): HTMLElement {
  const panel = el('div', { className: 'output-panel' });

  // Header: label + tabs + actions all in one compact bar
  const header = el('div', { className: 'output-panel__header' });
  const headerLabel = el('span', { className: 'output-panel__header-label', textContent: 'Output' });

  // Tabs
  const tabBar = el('div', { className: 'output-panel__tabs' });
  const tabs: { key: OutputTab; label: string; btn: HTMLButtonElement }[] = [
    { key: 'svg', label: 'Preview', btn: null! },
    { key: 'code', label: 'Code', btn: null! },
    { key: 'info', label: 'Info', btn: null! },
  ];

  for (const tab of tabs) {
    tab.btn = el('button', {
      className: `output-panel__tab${tab.key === store.get('activeTab') ? ' output-panel__tab--active' : ''}`,
      textContent: tab.label,
    }) as HTMLButtonElement;
    tab.btn.addEventListener('click', () => {
      store.set('activeTab', tab.key);
    });
    tabBar.appendChild(tab.btn);
  }

  store.subscribe('activeTab', (val) => {
    const active = val as OutputTab;
    for (const tab of tabs) {
      tab.btn.classList.toggle('output-panel__tab--active', tab.key === active);
    }
    renderContent(active);
  });

  // Action buttons (Download + Copy) inside header bar
  const downloadBtn = el('button', {
    className: 'output-panel__action-btn',
    textContent: '\u2913 Download',
  }) as HTMLButtonElement;
  const copyBtn = el('button', {
    className: 'output-panel__action-btn',
    textContent: '\u2398 Copy',
  }) as HTMLButtonElement;

  downloadBtn.disabled = true;
  copyBtn.disabled = true;

  downloadBtn.addEventListener('click', () => {
    const result = store.get('result');
    if (!result) return;
    const blob = new Blob([result.svg], { type: 'image/svg+xml' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    const name = store.get('fileName').replace(/\.[^.]+$/, '') || 'output';
    a.href = url;
    a.download = `${name}.svg`;
    a.click();
    URL.revokeObjectURL(url);
    showToast('SVG downloaded', 'success');
  });

  copyBtn.addEventListener('click', async () => {
    const result = store.get('result');
    if (!result) return;
    try {
      await navigator.clipboard.writeText(result.svg);
      showToast('SVG copied to clipboard', 'success');
    } catch {
      showToast('Failed to copy — check clipboard permissions', 'error');
    }
  });

  const headerSpacer = el('div', { className: 'output-panel__header-spacer' });
  const headerActions = el('div', { className: 'output-panel__actions' });
  headerActions.append(downloadBtn, copyBtn);
  header.append(headerLabel, tabBar, headerSpacer, headerActions);

  // Content area
  const content = el('div', { className: 'output-panel__content' });

  panel.append(header, content);

  // Render logic
  function renderContent(tab: OutputTab) {
    content.innerHTML = '';
    const result = store.get('result');

    if (!result) {
      renderEmpty(content);
      return;
    }

    switch (tab) {
      case 'svg':  renderSvg(content, result);  break;
      case 'code': renderCode(content, result); break;
      case 'info': renderInfo(content, result); break;
    }
  }

  function renderEmpty(container: HTMLElement) {
    const empty = el('div', { className: 'output-panel__empty' });
    empty.innerHTML = `
      <svg class="output-panel__empty-icon" viewBox="0 0 52 52" fill="none" xmlns="http://www.w3.org/2000/svg">
        <rect x="6" y="10" width="40" height="32" rx="5" stroke="currentColor" stroke-width="2"/>
        <circle cx="20" cy="24" r="4" stroke="currentColor" stroke-width="2"/>
        <path d="M6 36 l14-12 8 8 8-8 10 12" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        <path d="M32 16 l8 8 m0-8 l-8 8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <div class="output-panel__empty-title">No output yet</div>
      <div class="output-panel__empty-sub">Upload an image, then press <kbd>Vectorize</kbd><br>or use <kbd>Cmd</kbd>+<kbd>Enter</kbd></div>
    `;
    container.appendChild(empty);
  }

  function renderSvg(container: HTMLElement, result: VectorizeResponse) {
    const div = el('div', { className: 'output-panel__svg-preview' });
    div.innerHTML = result.svg;
    const svgEl = div.querySelector('svg');
    if (svgEl) {
      svgEl.style.width = '100%';
      svgEl.style.height = '100%';
      svgEl.style.display = 'block';
      if (!svgEl.getAttribute('viewBox') && svgEl.getAttribute('width') && svgEl.getAttribute('height')) {
        svgEl.setAttribute('viewBox', `0 0 ${svgEl.getAttribute('width')} ${svgEl.getAttribute('height')}`);
      }
      svgEl.removeAttribute('width');
      svgEl.removeAttribute('height');
    }
    container.appendChild(div);
  }

  function renderCode(container: HTMLElement, result: VectorizeResponse) {
    const pre = el('pre', { className: 'output-panel__code', textContent: result.svg });
    container.appendChild(pre);
  }

  function renderInfo(container: HTMLElement, result: VectorizeResponse) {
    const wrap = el('div', { className: 'output-panel__info' });
    const r = result.report;
    const m = r.metrics;
    const a = r.analysis;

    // Helper to rate a 0-1 score
    function rateClass(v: number): string {
      if (v >= 0.85) return 'good';
      if (v >= 0.65) return 'warn';
      return 'bad';
    }

    // Card 1: Image
    const imgCard = el('div', { className: 'metric-card' });
    imgCard.innerHTML = `
      <div class="metric-card__title">Image</div>
      <dl class="metric-kv-grid">
        <dt>Dimensions</dt><dd>${a.width} &times; ${a.height}</dd>
        <dt>Type</dt><dd>${a.image_type}</dd>
        <dt>Complexity</dt><dd>${a.complexity}</dd>
        <dt>Unique colors</dt><dd>${a.unique_colors.toLocaleString()}</dd>
        <dt>Edge density</dt><dd>${a.edge_density.toFixed(4)}</dd>
        <dt>Top-10 coverage</dt><dd>${(a.top_10_coverage * 100).toFixed(1)}%</dd>
      </dl>
    `;

    // Card 2: Vectorization settings
    const vecCard = el('div', { className: 'metric-card' });
    vecCard.innerHTML = `
      <div class="metric-card__title">Vectorization</div>
      <dl class="metric-kv-grid">
        <dt>Requested</dt><dd>${result.requested_method}</dd>
        <dt>Effective</dt><dd>${result.effective_method}</dd>
        <dt>Quality preset</dt><dd>${r.quality_preset}</dd>
        ${result.decision ? `<dt>Decision</dt><dd class="metric-tag">${result.decision.mode}</dd>` : ''}
        ${result.decision ? `<dt>Reason</dt><dd>${result.decision.reason}</dd>` : ''}
        <dt>Paths</dt><dd>${m.path_count.toLocaleString()}</dd>
        <dt>File size</dt><dd>${formatBytes(m.file_size)}</dd>
      </dl>
    `;

    // Card 3: Quality metrics with bars
    const qualCard = el('div', { className: 'metric-card' });
    function metricRow(label: string, value: number, fmt: (v: number) => string = (v) => v.toFixed(4)): string {
      const cls = rateClass(value);
      return `<div class="metric-row">
        <span class="metric-row__label">${label}</span>
        <span class="metric-row__value">${fmt(value)}</span>
        <div class="metric-row__bar"><div class="metric-row__bar-fill metric-row__bar-fill--${cls}" style="width:${(value * 100).toFixed(1)}%"></div></div>
      </div>`;
    }

    qualCard.innerHTML = `
      <div class="metric-card__title">Quality</div>
      ${metricRow('SSIM', m.ssim)}
      ${metricRow('Fidelity', m.fidelity_score)}
      ${metricRow('Edge F1', m.edge_f1)}
      ${metricRow('Color similarity', m.color_similarity)}
      ${metricRow('Foreground IoU', m.foreground_iou)}
      ${metricRow('Topology', m.topology_score)}
      <div class="metric-row">
        <span class="metric-row__label">PSNR</span>
        <span class="metric-row__value">${m.psnr.toFixed(1)} dB</span>
        <div class="metric-row__bar"><div class="metric-row__bar-fill metric-row__bar-fill--${m.psnr >= 35 ? 'good' : m.psnr >= 25 ? 'warn' : 'bad'}" style="width:${Math.min(m.psnr / 50 * 100, 100).toFixed(1)}%"></div></div>
      </div>
      <div class="metric-row">
        <span class="metric-row__label">Delta E</span>
        <span class="metric-row__value">${m.delta_e.toFixed(2)}</span>
        <div class="metric-row__bar"><div class="metric-row__bar-fill metric-row__bar-fill--${m.delta_e <= 3 ? 'good' : m.delta_e <= 10 ? 'warn' : 'bad'}" style="width:${Math.min((1 - m.delta_e / 50) * 100, 100).toFixed(1)}%"></div></div>
      </div>
    `;

    wrap.append(imgCard, vecCard, qualCard);
    container.appendChild(wrap);
  }

  // Update when result changes
  store.subscribe('result', () => {
    const result = store.get('result');
    downloadBtn.disabled = !result;
    copyBtn.disabled = !result;
    renderContent(store.get('activeTab'));
  });

  // Initial render
  renderContent(store.get('activeTab'));

  return panel;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1_048_576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1_048_576).toFixed(2)} MB`;
}
