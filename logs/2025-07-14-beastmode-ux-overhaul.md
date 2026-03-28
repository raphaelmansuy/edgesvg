# Task Log — 2025-07-14 UX Overhaul (resumed)

## Actions
- Updated `toolbar.ts`: replaced emoji dark-mode toggle with SVG moon/sun icon; replaced version badge with WASM status pill (loading→ready with fade-out)
- Updated `output-panel.ts`: replaced emoji empty state with SVG illustration + kbd hints; replaced table-based `renderInfo()` with metric cards (`metric-card`, `metric-kv-grid`, `metric-row` bars with good/warn/bad coloring)
- Added missing CSS: `.workspace`, `.sidebar`, `.sidebar__header`, `.sidebar__header-label`, `.sidebar__collapse-btn`, `.workspace--sidebar-collapsed` (collapse animation), `.params-panel__content`, `.pane-header__label`, `.pane-header__meta`, `.toolbar__btn--icon`
- Fixed `.params-panel` base styles (removed incorrect padding, added flex layout for content+footer split)

## Decisions
- Used Python script to write toolbar.ts (heredoc caused PTY corruption)
- Sidebar collapse animates via `width` transition on `.sidebar` (0.22s cubic-bezier)
- Collapsed sidebar: `width: 36px`, params-panel hidden via `opacity:0 + pointer-events:none`
- WASM status pill: fades to 50% opacity 4s after ready

## Next steps
- Connect browser MCP extension to do visual inspection at http://localhost:5175
- Verify sidebar collapse toggle works visually
- Verify metric cards render correctly with real data

## Lessons/insights
- CSS for workspace/sidebar layout was never added in previous session (only responsive overrides existed)
- Heredoc in zsh terminal with complex content → use Python file-write approach instead
