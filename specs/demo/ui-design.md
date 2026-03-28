# UI Design

## Design Principles

1. **Minimalist** — white space, clear hierarchy, zero visual noise
2. **Functional** — every pixel serves a purpose
3. **Accessible** — WCAG AA contrast, keyboard nav, focus rings
4. **Responsive** — stacked layout on narrow viewports

## Screen Layout

### Desktop (≥ 768px) — Split Pane

```
┌──────────────────────────────────────────────────────────────────────┐
│  Toolbar                                                             │
│  ┌──────────┐                         ┌────────┐ ┌──┐ ┌──┐ ┌──┐    │
│  │ ES  Logo │  EdgeSVG Demo           │Upload  │ │v │ │🌙│ │⚙ │    │
│  └──────────┘                         └────────┘ └──┘ └──┘ └──┘    │
├──────────────────────────────────────────────────────────────────────┤
│  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ Progress Bar (3px, shown during vectorization)    │
├───────────────────────────┬──┬───────────────────────────────────────┤
│                           │  │                                       │
│   Left Pane               │▐▐│   Right Pane                          │
│   ┌─────────────────────┐ │▐▐│   ┌───────────────────────────────┐   │
│   │                     │ │▐▐│   │  ┌─────┐ ┌────────┐ ┌──────┐ │   │
│   │  Source Image /     │ │▐▐│   │  │ SVG │ │  Code  │ │ Info │ │   │
│   │  Drop Zone          │ │▐▐│   │  └─────┘ └────────┘ └──────┘ │   │
│   │                     │ │▐▐│   ├───────────────────────────────┤   │
│   │  ┌───────────────┐  │ │▐▐│   │                               │   │
│   │  │               │  │ │▐▐│   │  SVG Preview                  │   │
│   │  │    Image      │  │ │▐▐│   │  (rendered inline)            │   │
│   │  │   Preview     │  │ │▐▐│   │                               │   │
│   │  │               │  │ │▐▐│   │       or                      │   │
│   │  └───────────────┘  │ │▐▐│   │                               │   │
│   │                     │ │▐▐│   │  SVG Code                     │   │
│   │  ┌───────────────┐  │ │▐▐│   │  (monospace, line numbers)    │   │
│   │  │  Parameters   │  │ │▐▐│   │                               │   │
│   │  │  Panel        │  │ │▐▐│   │       or                      │   │
│   │  │  ┌─────────┐  │  │ │▐▐│   │                               │   │
│   │  │  │ Method  │  │  │ │▐▐│   │  Analysis Info                │   │
│   │  │  ├─────────┤  │  │ │▐▐│   │  (metrics, report)           │   │
│   │  │  │ Quality │  │  │ │▐▐│   │                               │   │
│   │  │  ├─────────┤  │  │ │▐▐│   ├───────────────────────────────┤   │
│   │  │  │ SSIM    │  │  │ │▐▐│   │  Actions                     │   │
│   │  │  ├─────────┤  │  │ │▐▐│   │  ┌──────────┐ ┌───────────┐  │   │
│   │  │  │ Colors  │  │  │ │▐▐│   │  │ Download │ │   Copy    │  │   │
│   │  │  ├─────────┤  │  │ │▐▐│   │  │   SVG    │ │   SVG     │  │   │
│   │  │  │ [Vecz!] │  │  │ │▐▐│   │  └──────────┘ └───────────┘  │   │
│   │  │  └─────────┘  │  │ │▐▐│   └───────────────────────────────┘   │
│   │  └───────────────┘  │ │▐▐│                                       │
│   └─────────────────────┘ │▐▐│                                       │
│                           │▐▐│                                       │
├───────────────────────────┴──┴───────────────────────────────────────┤
│  Toast Notifications (bottom-right, stacked)                         │
└──────────────────────────────────────────────────────────────────────┘
```

### Mobile (< 768px) — Stacked

```
┌─────────────────────────┐
│  Toolbar (wrapped)      │
├─────────────────────────┤
│  ▓▓▓ Progress ▓▓▓▓▓▓▓▓ │
├─────────────────────────┤
│                         │
│  Source Image /         │
│  Drop Zone              │
│  + Parameters           │
│                         │
├═════════════════════════┤  ◀── drag handle (horizontal)
│                         │
│  SVG Preview / Code     │
│  + Download buttons     │
│                         │
└─────────────────────────┘
```

## Drop Zone (Empty State)

```
┌───────────────────────────────────────┐
│                                       │
│         ┌───────────────────┐         │
│         │                   │         │
│         │    ┌─────────┐    │         │
│         │    │  ▲ icon  │    │         │
│         │    └─────────┘    │         │
│         │                   │         │
│         │  Drop an image    │         │
│         │     here          │         │
│         │                   │         │
│         │  or click to      │         │
│         │  browse           │         │
│         │                   │         │
│         │  PNG, JPG, WebP,  │         │
│         │  GIF, BMP         │         │
│         │                   │         │
│         └─ ─ ─ ─ ─ ─ ─ ─ ─┘         │
│          (dashed border)              │
│                                       │
└───────────────────────────────────────┘
```

## Parameters Panel

```
┌─────────────────────────────────┐
│  Parameters                     │
│ ────────────────────────────── │
│                                 │
│  Method                         │
│  ┌────────────────────────────┐ │
│  │ auto ▾                     │ │
│  └────────────────────────────┘ │
│                                 │
│  Quality                        │
│  ┌────┐ ┌────┐ ┌────┐ ┌─────┐  │
│  │Figm│ │Bal │ │Qual│ │Ultra│  │
│  └────┘ └────┘ └────┘ └─────┘  │
│                                 │
│  Target SSIM                    │
│  ○───────────────────●──○ 0.998 │
│  0.90              1.0          │
│                                 │
│  Max Colors (optional)          │
│  ┌────────────────────────────┐ │
│  │                            │ │
│  └────────────────────────────┘ │
│                                 │
│  Max Iterations                 │
│  ○────●──────────────────○  4   │
│  1                      10      │
│                                 │
│  Max File Size                  │
│  ○─────────────●─────────○ 100K │
│  10K                   500K     │
│                                 │
│  ┌────────────────────────────┐ │
│  │      ▶ Vectorize           │ │
│  └────────────────────────────┘ │
└─────────────────────────────────┘
```

## Output Tabs

```
┌────────────────────────────────────────────────┐
│  ┌─────────┐ ┌──────────┐ ┌──────────┐        │
│  │▓▓ SVG ▓▓│ │   Code   │ │   Info   │        │
│  └─────────┘ └──────────┘ └──────────┘        │
│ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │
│                                                │
│  [SVG tab]  →  Rendered SVG preview inline     │
│  [Code tab] →  Raw SVG source (monospace)      │
│  [Info tab] →  Metrics table (SSIM, paths,     │
│                file size, method, etc.)         │
│                                                │
├────────────────────────────────────────────────┤
│  ┌────────────┐  ┌──────────────┐              │
│  │ ↓ Download │  │ ⎘ Copy SVG   │              │
│  └────────────┘  └──────────────┘              │
└────────────────────────────────────────────────┘
```

## Design Tokens

Following the edgeparse pattern, all visual values live in CSS custom properties:

```css
:root {
  /* Brand */
  --es-primary:        #2563eb;
  --es-primary-hover:  #1d4ed8;
  --es-primary-light:  rgba(37, 99, 235, 0.08);

  /* Neutrals */
  --es-bg:             #ffffff;
  --es-fg:             #0f172a;
  --es-surface:        #f8fafc;
  --es-surface2:       #f1f5f9;
  --es-border:         #e2e8f0;
  --es-muted:          #64748b;

  /* Semantic */
  --es-error:          #dc2626;
  --es-success:        #16a34a;

  /* Geometry */
  --es-radius:         8px;
  --es-radius-lg:      16px;

  /* Typography */
  --es-font-sans:      system-ui, -apple-system, sans-serif;
  --es-font-mono:      ui-monospace, 'Cascadia Code', monospace;
}
```

Dark mode overrides via `.dark` class on `<html>`:

```css
.dark {
  --es-bg:       #0f172a;
  --es-fg:       #f1f5f9;
  --es-surface:  #1e293b;
  --es-surface2: #0f172a;
  --es-border:   #334155;
  --es-muted:    #94a3b8;
}
```

## Component Hierarchy

```
app-shell
├── toolbar
│   ├── brand (logo + title)
│   ├── upload button (label + hidden input)
│   ├── version badge
│   └── dark mode toggle
├── progress-bar
└── split-pane
    ├── left-pane
    │   ├── drop-zone  (shown when no image)
    │   ├── image-preview (shown when image loaded)
    │   └── params-panel (always visible below image)
    └── right-pane
        ├── output-tabs (SVG | Code | Info)
        ├── output-content
        └── action-bar (download + copy)
```

## Cross-References

- Component file layout → [Implementation Plan](./implementation-plan.md)
- WASM parameters → [API Contract](./api-contract.md)
- Architecture overview → [Architecture](./architecture.md)
