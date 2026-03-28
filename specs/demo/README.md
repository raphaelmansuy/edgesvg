# EdgeSVG Demo — Specification

> Local HTML5 application for raster-to-SVG vectorization powered by the
> EdgeSVG WebAssembly module. All processing happens client-side — no server
> required.

## Documents

| Document | Description |
|---|---|
| [Architecture](./architecture.md) | System architecture, data flow, component map |
| [UI Design](./ui-design.md) | Screen layout, wireframes, design tokens |
| [API Contract](./api-contract.md) | WASM API surface, request/response shapes |
| [Implementation Plan](./implementation-plan.md) | File tree, build steps, dev workflow |

## Quick Reference

- **Stack**: Vite + TypeScript + edgesvg-wasm (no framework)
- **Pattern**: Component functions returning DOM elements, reactive state store
- **Design**: Minimalist, light/dark mode, responsive, WCAG AA contrast
- **Processing**: Web Worker for non-blocking WASM calls
