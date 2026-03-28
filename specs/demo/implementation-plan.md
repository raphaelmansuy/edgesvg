# Implementation Plan

## File Tree

```
demo/
├── index.html                   HTML entry point
├── package.json                 Dependencies: vite, typescript
├── tsconfig.json                TypeScript config
├── vite.config.ts               Vite config (WASM exclusion, COOP/COEP headers)
├── public/
│   └── favicon.svg              App icon
└── src/
    ├── main.ts                  Mount app, pre-warm WASM, wire subscriptions
    ├── state.ts                 Reactive EventTarget state store
    ├── types.ts                 Shared TypeScript interfaces
    ├── style.css                Design tokens + all component styles
    ├── components/
    │   ├── app-shell.ts         Layout: toolbar + progress + split-pane
    │   ├── toolbar.ts           Brand, upload button, dark mode, version
    │   ├── drop-zone.ts         Drag-and-drop / click-to-browse zone
    │   ├── params-panel.ts      Method, quality, SSIM, colors, iterations
    │   ├── preview-pane.ts      Side-by-side: original image + SVG output
    │   ├── output-panel.ts      Tabs (SVG | Code | Info), download/copy
    │   ├── split-pane.ts        Resizable left/right split
    │   ├── progress-bar.ts      Thin animated bar
    │   └── toast.ts             Notification system
    ├── services/
    │   └── wasm-bridge.ts       Init WASM, manage Worker, expose async API
    ├── workers/
    │   └── vectorize-worker.ts  Load edgesvg-wasm, handle messages
    └── utils/
        └── dom.ts               DOM helper (el() factory)
```

## Build & Development

### Prerequisites

```bash
# 1. Install wasm-pack (if not present)
cargo install wasm-pack

# 2. Build the WASM package
wasm-pack build crates/edgesvg-wasm --target bundler --out-dir ../../demo/wasm-pkg

# 3. Install demo dependencies
cd demo && npm install

# 4. Dev server with HMR
npm run dev

# 5. Production build
npm run build
```

### Vite Configuration

```
┌─────────────────────────────────────────────────────────┐
│  vite.config.ts                                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  optimizeDeps.exclude: ['edgesvg-wasm']                 │
│  ← Prevent Vite from pre-bundling the WASM package      │
│                                                         │
│  build.target: 'esnext'                                 │
│  ← Required for top-level await, WASM ESM imports       │
│                                                         │
│  server.headers:                                        │
│    Cross-Origin-Opener-Policy: same-origin              │
│    Cross-Origin-Embedder-Policy: require-corp           │
│  ← Required for SharedArrayBuffer (Web Worker)          │
│                                                         │
│  server.fs.allow: ['..']                                │
│  ← Allow importing wasm-pkg from parent directory       │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Dependency Graph

```
index.html
    └── src/main.ts
        ├── src/style.css
        ├── src/state.ts ◀──────────── shared by all components
        ├── src/components/app-shell.ts
        │   ├── src/components/toolbar.ts
        │   │   └── (triggers file upload → state.imageBytes)
        │   ├── src/components/progress-bar.ts
        │   └── src/components/split-pane.ts
        │       ├── Left: drop-zone.ts / preview + params-panel.ts
        │       └── Right: output-panel.ts
        └── src/services/wasm-bridge.ts
            └── src/workers/vectorize-worker.ts
                └── wasm-pkg/edgesvg_wasm.js
                    └── edgesvg_wasm_bg.wasm
```

## Implementation Sequence

```
Phase 1: Scaffold                    Phase 2: Core               Phase 3: Polish
─────────────────                    ──────────────               ───────────────
                                                                        
 ┌──────────────┐                    ┌──────────────┐            ┌──────────────┐
 │ package.json │                    │ wasm-bridge  │            │ dark mode    │
 │ vite.config  │                    │ + worker     │            │ responsive   │
 │ tsconfig     │                    │              │            │ polish       │
 │ index.html   │                    │ vectorize()  │            │              │
 └──────┬───────┘                    │ analyze()    │            │ toast system │
        │                            └──────┬───────┘            │ error states │
        ▼                                   │                    └──────────────┘
 ┌──────────────┐                           ▼                          
 │ style.css    │                    ┌──────────────┐            
 │ state.ts     │                    │ params-panel │            
 │ types.ts     │                    │ output-panel │            
 │ dom.ts       │                    │ preview-pane │            
 └──────┬───────┘                    └──────────────┘            
        │                                                        
        ▼                                                        
 ┌──────────────┐                                                
 │ app-shell    │                                                
 │ toolbar      │                                                
 │ drop-zone    │                                                
 │ split-pane   │                                                
 │ progress-bar │                                                
 └──────────────┘                                                
```

## Makefile Integration

Add to root `Makefile`:

```makefile
demo-wasm:
	wasm-pack build crates/edgesvg-wasm --target bundler --out-dir ../../demo/wasm-pkg

demo-dev: demo-wasm
	cd demo && npm install && npm run dev

demo-build: demo-wasm
	cd demo && npm install && npm run build
```

## Cross-References

- Architecture overview → [Architecture](./architecture.md)
- UI wireframes → [UI Design](./ui-design.md)
- WASM API surface → [API Contract](./api-contract.md)
