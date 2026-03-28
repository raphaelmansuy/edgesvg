# API Contract

## WASM Exports

The `edgesvg-wasm` package exposes these functions after `init()`:

```
┌──────────────────────────────────────────────────────────────────┐
│                     edgesvg-wasm Exports                         │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  init()              → Promise<void>   (WASM bootstrap)          │
│                                                                  │
│  vectorize(          → VectorizeResponse                         │
│    input: Uint8Array,                                            │
│    request: VectorizeRequest | null                              │
│  )                                                               │
│                                                                  │
│  analyze(            → AnalyzeResponse                           │
│    input: Uint8Array                                             │
│  )                                                               │
│                                                                  │
│  compare(            → QualityMetrics                            │
│    input: Uint8Array,                                            │
│    svg: string                                                   │
│  )                                                               │
│                                                                  │
│  optimize_svg(       → OptimizeResponse                          │
│    svg: string,                                                  │
│    precision?: number                                            │
│  )                                                               │
│                                                                  │
│  render(             → Uint8Array (PNG bytes)                    │
│    svg: string,                                                  │
│    width: number,                                                │
│    height: number                                                │
│  )                                                               │
│                                                                  │
│  version()           → string                                    │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## VectorizeRequest

Sent as a JS object; `serde_wasm_bindgen` deserializes it into Rust.

```typescript
interface VectorizeRequest {
  method?:         VectorizeMethod;  // default: "hifi"
  target_ssim?:    number;           // default: 0.998
  max_file_size?:  number;           // default: 100_000
  max_iterations?: number;           // default: 4
  quality?:        QualityPreset;    // default: "ultra"
  logo_quality?:   LogoQualityPreset | null;
  colors?:         number | null;    // optional color limit
}
```

### VectorizeMethod enum

```
┌──────────┬────────────────────────────────────────────────────┐
│ Value    │ Description                                        │
├──────────┼────────────────────────────────────────────────────┤
│ "hifi"   │ Default high-fidelity tracing                     │
│ "logo"   │ Optimized for logos / icons                       │
│ "premium"│ Premium quality with SSIM targeting                │
│ "auto"   │ Auto-detect image type and pick best method       │
│ "smart"  │ Iterative refinement with SSIM + size targets     │
│ "optimal"│ Balanced quality/size optimization                │
│ "bayesian"│ Bayesian-enhanced smart vectorization            │
│ "sam"    │ Segment Anything fallback (→ auto)                │
└──────────┴────────────────────────────────────────────────────┘
```

### QualityPreset enum

```
┌───────────┬──────────────────────────────────────────────────┐
│ Value     │ Description                                      │
├───────────┼──────────────────────────────────────────────────┤
│ "figma"   │ Fastest, lighter output, optimizer precision 1   │
│ "balanced"│ Good balance of quality and file size             │
│ "quality" │ High fidelity, larger files                      │
│ "ultra"   │ Maximum quality, no compromises                  │
└───────────┴──────────────────────────────────────────────────┘
```

### LogoQualityPreset enum

```
┌───────────┬──────────────────────────────────────────────────┐
│ Value     │ Description                                      │
├───────────┼──────────────────────────────────────────────────┤
│ "clean"   │ Minimal path count, simple shapes                │
│ "balanced"│ Default for logo method                          │
│ "high"    │ More detail preserved                            │
│ "ultra"   │ Maximum logo detail                              │
└───────────┴──────────────────────────────────────────────────┘
```

## VectorizeResponse

```typescript
interface VectorizeResponse {
  svg:              string;              // The SVG output
  report: {
    analysis: {
      width:           number;
      height:          number;
      unique_colors:   number;
      top_10_coverage: number;
      top_50_coverage: number;
      color_variance:  number;
      edge_density:    number;
      dominant_colors: string[];
      image_type:      "logo" | "icon" | "illustration" | "photo";
      complexity:      "simple" | "medium" | "complex";
    };
    settings: {
      color_mode:          string;
      hierarchical:        string;
      mode:                "spline" | "polygon";
      filter_speckle:      number;
      color_precision:     number;
      layer_difference:    number;
      length_threshold:    number;
      corner_threshold:    number;
      max_iterations:      number;
      splice_threshold:    number;
      path_precision:      number;
      optimizer_precision: number;
    };
    quality_preset: string;
    metrics: QualityMetrics;
  };
  requested_method:  string;
  effective_method:  string;
  fallback_from:     string | null;
  decision:          AutoDecision | null;
}
```

## AnalyzeResponse

```typescript
interface AnalyzeResponse {
  analysis: ImageAnalysis;   // same shape as report.analysis above
  decision: AutoDecision;
}

interface AutoDecision {
  mode:   "logo" | "premium";
  reason: string;
}
```

## QualityMetrics

```typescript
interface QualityMetrics {
  ssim:             number;   // Structural similarity (0-1)
  ssim_perceptual:  number;   // Perceptual SSIM
  edge_similarity:  number;   // Edge matching score
  edge_precision:   number;
  edge_recall:      number;
  edge_f1:          number;
  foreground_iou:   number;   // Intersection over Union
  color_similarity: number;
  fidelity_score:   number;   // Composite fidelity (0-1)
  delta_e:          number;   // Color difference
  topology_score:   number;
  psnr:             number;   // Peak signal-to-noise ratio
  mae:              number;   // Mean absolute error
  file_size:        number;   // SVG file size in bytes
  path_count:       number;   // Number of SVG paths
}
```

## Worker Protocol

```
┌────────────┐                    ┌──────────────────┐
│ Main Thread │                    │   Web Worker     │
│             │                    │                  │
│             │ ── {type:'init'} ──▶                  │
│             │                    │ load WASM        │
│             │ ◀── {type:'ready'}─│                  │
│             │                    │                  │
│             │ ── {type:'vectorize', ──▶              │
│             │     id, bytes,     │ call vectorize() │
│             │     request}       │                  │
│             │                    │                  │
│             │ ◀── {type:'result',─│                  │
│             │     id, ok, data}  │                  │
│             │                    │                  │
│             │ ── {type:'analyze',──▶                 │
│             │     id, bytes}     │ call analyze()   │
│             │                    │                  │
│             │ ◀── {type:'result',─│                  │
│             │     id, ok, data}  │                  │
└────────────┘                    └──────────────────┘
```

## Cross-References

- Architecture overview → [Architecture](./architecture.md)
- How params map to UI controls → [UI Design](./ui-design.md)
- Build instructions → [Implementation Plan](./implementation-plan.md)
