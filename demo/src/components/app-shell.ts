/** AppShell — top-level layout: toolbar + progress + workspace (sidebar | source | svg). */

import { el } from '../utils/dom';
import { createToolbar } from './toolbar';
import { createDropZone, enableDragDrop } from './drop-zone';
import { createOutputPanel } from './output-panel';
import { createSplitPane } from './split-pane';
import { createProgressBar } from './progress-bar';
import { createParamsPanel } from './params-panel';
import { initToastListener } from './toast';

export function mountApp(root: HTMLElement): void {
  root.innerHTML = '';
  root.className = 'app';

  const toolbar = createToolbar();
  const progressBar = createProgressBar();

  // Sidebar with header + collapse toggle
  const sidebar = el('aside', { className: 'sidebar' });

  const sidebarHeader = el('div', { className: 'sidebar__header' });
  const sidebarLabel = el('span', { className: 'sidebar__header-label', textContent: 'Parameters' });
  const collapseBtn = el('button', {
    className: 'sidebar__collapse-btn',
    ariaLabel: 'Collapse sidebar',
    title: 'Toggle sidebar',
    textContent: '‹',
  }) as HTMLButtonElement;

  sidebarHeader.append(sidebarLabel, collapseBtn);
  sidebar.append(sidebarHeader, createParamsPanel());

  // Workspace with collapse state
  const workspace = el('div', { className: 'workspace' });
  let collapsed = false;

  collapseBtn.addEventListener('click', () => {
    collapsed = !collapsed;
    workspace.classList.toggle('workspace--sidebar-collapsed', collapsed);
    collapseBtn.textContent = collapsed ? '›' : '‹';
    collapseBtn.setAttribute('aria-label', collapsed ? 'Expand sidebar' : 'Collapse sidebar');
  });

  // Split canvas: source image pane | SVG output pane
  const sourcePane = createDropZone();
  const outputPane = createOutputPanel();
  const splitPane = createSplitPane(sourcePane, outputPane);

  workspace.append(sidebar, splitPane);
  root.append(toolbar, progressBar, workspace);

  enableDragDrop();
  initToastListener();
}
