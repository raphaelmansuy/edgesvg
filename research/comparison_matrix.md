# Vectorization Tools & Algorithms Comparison Matrix

Quick reference for comparing SOTA vectorization approaches, libraries, and tools.

---

## 1. Algorithm Comparison

| Algorithm | Type | Complexity | Input | Output Quality | Editability | Speed | Best For |
|-----------|------|------------|-------|----------------|-------------|-------|----------|
| **Potrace** | Classical | O(n²) | Binary | Good | High | Fast | Logos, simple graphics |
| **vtracer** | Classical | O(n) | Color | Good | High | Very Fast | General purpose |
| **DiffVG** | Optimization | O(iter) | RGB | Excellent | Medium | Slow | High fidelity |
| **LIVE** | Layer+Opt | O(n×iter) | RGB | Very Good | High | Slow | Editable output |
| **VectorFusion** | SDS | O(iter) | Text/RGB | Good | Low | Very Slow | Text-guided |
| **SVGDreamer** | VSD | O(iter) | Text/RGB | Very Good | Medium | Very Slow | Complex scenes |
| **Layered Vec.** | SAM+SDS | O(n×iter) | RGB | Excellent | High | Slow | Best overall |
| **SVGFusion** | Diffusion | O(1) | Text | Excellent | High | Fast | Text-to-SVG |

---

## 2. Library Comparison

| Library | Language | Stars | License | Python Support | GPU | Maintained |
|---------|----------|-------|---------|----------------|-----|------------|
| **vtracer** | Rust | 5.1k | MIT | ✅ Yes | ❌ | ✅ Active |
| **Potrace** | C | N/A | GPL | ✅ pypotrace | ❌ | ⚠️ Stable |
| **DiffVG** | C++/CUDA | 1.2k | MIT | ✅ Yes | ✅ | ⚠️ Moderate |
| **LIVE** | Python | 1k+ | Apache | ✅ Yes | ✅ | ⚠️ Moderate |
| **SVGO** | JavaScript | 22.1k | MIT | ✅ Subprocess | ❌ | ✅ Active |
| **simplify-js** | JavaScript | 2.4k | BSD | ✅ simplify.py | ❌ | ⚠️ Stable |
| **Primitive** | Go | 9k+ | MIT | ❌ | ❌ | ⚠️ Stable |
| **SAM** | Python | 45k+ | Apache | ✅ Yes | ✅ | ✅ Active |

---

## 3. Feature Matrix

| Feature | Potrace | vtracer | DiffVG | LIVE | VectorFusion | SVGFusion |
|---------|---------|---------|--------|------|--------------|-----------|
| Color Support | ❌ Binary | ✅ Full | ✅ Full | ✅ Full | ✅ Full | ✅ Full |
| Gradient Support | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ |
| Shape Primitives | ❌ Paths only | ❌ Paths only | ✅ All | ✅ Paths | ✅ Paths | ✅ All |
| Layered Output | ❌ | ✅ Stacked | ❌ | ✅ | ❌ | ✅ |
| Text-guided | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| Semantic Awareness | ❌ | ❌ | ❌ | ⚠️ Partial | ✅ | ✅ |
| GPU Required | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ |

---

## 4. Performance Benchmarks

### 4.1 Speed (1024×1024 image, RTX 3080)

| Method | Time | Notes |
|--------|------|-------|
| vtracer | <1s | CPU only |
| Potrace | 1-2s | CPU only |
| SAM + Potrace | 2-5s | GPU for SAM |
| DiffVG (500 iter) | 30-60s | GPU required |
| LIVE | 5-10min | GPU required |
| VectorFusion | 10-30min | GPU required |
| SVGFusion | 1-2s | GPU required |

### 4.2 Quality (SSIM on test set)

| Method | SSIM | PSNR (dB) | Path Count |
|--------|------|-----------|------------|
| vtracer | 0.92 | 28 | ~500 |
| Potrace | 0.85 | 24 | ~200 |
| DiffVG | 0.97 | 35 | ~100 |
| LIVE | 0.95 | 32 | ~150 |
| Layered Vec. | 0.98 | 38 | ~80 |

---

## 5. Use Case Recommendations

### Simple Logos/Icons
**Recommended**: vtracer or Potrace
- Fast processing
- Clean output
- No GPU needed

### High-Fidelity Reproduction
**Recommended**: DiffVG + SAM segmentation
- Best visual quality
- Gradient-based optimization
- GPU required

### Editable Output (Design Tools)
**Recommended**: LIVE or Layered Vectorization
- Layered structure
- Semantic grouping
- Easy to edit in Illustrator/Figma

### Text-to-SVG Generation
**Recommended**: SVGFusion
- Fastest text-to-SVG
- High quality output
- Good editability

### Production Pipeline
**Recommended**: vtracer + SVGO
- Fast and reliable
- Good quality/speed trade-off
- Easy to integrate

---

## 6. Integration Difficulty

| Integration | Difficulty | Requirements | Notes |
|-------------|------------|--------------|-------|
| vtracer | Easy | `pip install vtracer` | Works out of box |
| Potrace | Easy | `pip install pypotrace` + potrace binary | System dependency |
| SVGO | Easy | Node.js | subprocess call |
| DiffVG | Medium | CUDA toolkit, build from source | Complex build |
| LIVE | Medium | DiffVG + PyTorch | Depends on DiffVG |
| SAM/SAM2 | Easy | `pip install segment-anything` | Pre-trained weights |
| SVGFusion | Hard | Custom training, large dataset | Research code |

---

## 7. Commercial Tool Comparison

| Tool | Price | API | Quality | Speed | Features |
|------|-------|-----|---------|-------|----------|
| **Vectorizer.ai** | $9.99/mo | ✅ REST | Excellent | Fast | Full shape fitting, AI |
| **Vector Magic** | $295 once | ❌ | Very Good | Fast | Bayesian, desktop |
| **Adobe Image Trace** | $22.99/mo | ❌ | Good | Fast | Part of Illustrator |
| **Inkscape Trace** | Free | ❌ | Moderate | Slow | Uses Potrace |

---

## 8. SVG Optimization Comparison

| Tool | Type | Size Reduction | Quality Preservation | Speed |
|------|------|----------------|---------------------|-------|
| **SVGO** | Node.js | 30-70% | Excellent | Fast |
| **scour** | Python | 20-50% | Good | Fast |
| **svgcleaner** | Rust | 30-60% | Good | Very Fast |
| **nano** | Web | 40-60% | Good | N/A |

---

## 9. Decision Matrix

Choose based on your priorities:

```
Speed is Critical? 
  └─Yes→ vtracer
  └─No→ Quality is Critical?
          └─Yes→ GPU Available?
                  └─Yes→ DiffVG or LIVE
                  └─No→ vtracer with high settings
          └─No→ Editability is Critical?
                  └─Yes→ LIVE
                  └─No→ vtracer
```

---

## 10. Current Vectalab Position

### Current Stack
- **Segmentation**: SAM (ViT-B, ViT-H)
- **Tracing**: Potrace
- **Optimization**: Bayesian refinement
- **High-fidelity**: vtracer

### Recommended Upgrades

| Priority | Upgrade | Impact | Effort |
|----------|---------|--------|--------|
| 1 | Add SVGO post-processing | File size -50% | Low |
| 2 | Upgrade to SAM2 | Better edges | Low |
| 3 | Add shape detection | Cleaner SVG | Medium |
| 4 | Integrate DiffVG | Quality +10% | High |
| 5 | Implement layered approach | Editability | High |

---

*Quick reference for Vectalab vectorization research*
*Last updated: June 2025*
