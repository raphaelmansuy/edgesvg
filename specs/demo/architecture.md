# Architecture

## Overview

The EdgeSVG Demo is a single-page application that runs entirely in the
browser. It loads the `edgesvg-wasm` WebAssembly module, accepts raster images
(PNG, JPG, WebP, GIF, BMP) via upload or drag-and-drop, vectorizes them
client-side, and lets the user preview, compare, and download the resulting SVG.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Browser (Main Thread)                            │
│                                                                         │
│  ┌──────────┐   ┌──────────┐   ┌───────────────┐   ┌────────────────┐  │
│  │  main.ts  │──▶│  state.ts │──▶│  Components   │──▶│  DOM / UI      │  │
│  │ (entry)   │   │ (store)  │   │  (app-shell,  │   │  (rendered     │  │
│  └─────┬─────┘   └────▲─────┘   │   toolbar,    │   │   HTML)        │  │
│        │               │         │   drop-zone,  │   └────────────────┘  │
│        │               │         │   params,     │                       │
│        │               │         │   preview,    │                       │
│        │               │         │   output)     │                       │
│        │               │         └───────────────┘                       │
│        │               │                                                 │
│        ▼               │                                                 │
│  ┌─────────────┐       │                                                 │
│  │ wasm-bridge │───────┘  state updates on result                        │
│  │  (service)  │                                                         │
│  └──────┬──────┘                                                         │
│         │ postMessage                                                    │
├─────────┼────────────────────────────────────────────────────────────────┤
│         ▼                                                                │
│  ┌──────────────────┐                                                    │
│  │  Web Worker       │                                                   │
│  │  ┌──────────────┐ │                                                   │
│  │  │ edgesvg-wasm │ │  ◀── WASM module loaded once                     │
│  │  │  .wasm       │ │                                                   │
│  │  └──────────────┘ │                                                   │
│  └──────────────────┘                                                    │
└─────────────────────────────────────────────────────────────────────────┘
```

## Component Map

```
src/
├── main.ts                  Entry point: mounts app, pre-warms WASM
├── state.ts                 Reactive state store (EventTarget)
├── types.ts                 Shared TypeScript interfaces
├── style.css                Design tokens + all component styles
│
├── components/
│   ├── app-shell.ts         Top-level layout: toolbar + split-pane
│   ├── toolbar.ts           Header bar: brand, upload, dark mode
│   ├── drop-zone.ts         Drag-and-drop / click-to-upload zone
│   ├── params-panel.ts      Vectorization parameters sidebar
│   ├── preview-pane.ts      Side-by-side original ↔ SVG preview
│   ├── output-panel.ts      SVG code viewer + download
│   ├── progress-bar.ts      Thin animated progress indicator
│   └── toast.ts             Notification toasts (errors, info)
│
├── services/
│   └── wasm-bridge.ts       WASM init + worker communication
│
└── workers/
    └── vectorize-worker.ts  Runs edgesvg-wasm in a Web Worker
```

## Data Flow

```
                   ┌─────────────┐
                   │  User drops  │
                   │  image file  │
                   └──────┬──────┘
                          │
                          ▼
                   ┌─────────────┐
                   │ Read bytes  │  (FileReader API)
                   │ into U8Arr  │
                   └──────┬──────┘
                          │
                ┌─────────▼──────────┐
                │  store.set(        │
                │    'imageBytes',   │
                │    bytes           │
                │  )                 │
                └─────────┬──────────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
         ▼                ▼                ▼
  ┌────────────┐  ┌──────────────┐  ┌───────────────┐
  │ Preview     │  │ Analyze via  │  │ Vectorize via │
  │ original    │  │ WASM bridge  │  │ WASM bridge   │
  │ (img tag)   │  │  (optional)  │  │  (worker)     │
  └────────────┘  └──────┬───────┘  └───────┬───────┘
                         │                   │
                         ▼                   ▼
                  ┌────────────┐      ┌────────────┐
                  │ store.set( │      │ store.set( │
                  │ 'analysis')│      │ 'result')  │
                  └─────┬──────┘      └──────┬─────┘
                        │                    │
                        ▼                    ▼
                 ┌─────────────┐     ┌──────────────┐
                 │ Show image  │     │ Render SVG   │
                 │ metadata    │     │ in preview   │
                 └─────────────┘     │ + code view  │
                                     │ + download   │
                                     └──────────────┘
```

## State Machine

```
                          ┌──────┐
                          │ idle │ ◀─────── app start
                          └──┬───┘
                             │ user uploads image
                             ▼
                       ┌───────────┐
                       │ analyzing │    (optional quick pass)
                       └─────┬─────┘
                             │
                             ▼
                      ┌─────────────┐
              ┌───────│ vectorizing │
              │       └──────┬──────┘
              │              │ success
              │              ▼
              │        ┌──────────┐
              │        │   done   │──── user changes params ───▶ vectorizing
              │        └──────────┘
              │              │ user uploads new image
              │              ▼
              │        ┌──────────┐
              │        │   idle   │
              │        └──────────┘
              │
              │ error
              ▼
         ┌─────────┐
         │  error   │──── user retries / uploads new ───▶ idle
         └─────────┘
```

## Technology Choices

| Concern | Choice | Rationale |
|---|---|---|
| Bundler | Vite | Fast HMR, native ESM, WASM support |
| Language | TypeScript | Type safety, IDE support |
| UI Framework | None (vanilla) | Minimal footprint, mirrors edgeparse pattern |
| State | EventTarget store | Lightweight reactive pattern, no deps |
| WASM execution | Web Worker | Non-blocking UI during vectorization |
| Styling | CSS custom properties | Consistent design tokens, dark mode |

## Cross-References

- WASM API surface → [API Contract](./api-contract.md)
- UI wireframes → [UI Design](./ui-design.md)
- File tree + build → [Implementation Plan](./implementation-plan.md)
