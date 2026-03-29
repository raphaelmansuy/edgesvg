---
title: Vectorization Modes
description: Understand the different EdgeSVG vectorization modes — convert, logo, premium, auto, smart, and optimal — and when to use each.
---

EdgeSVG exposes several vectorization modes, each tuned for different image types and quality requirements. All modes produce scored output with the same `VectorizeResponse` contract.

## Mode Reference

### `auto` — Recommended for most cases

EdgeSVG analyzes the image and delegates the choice to the engine.

**When to use:** You don't know what kind of image you have, or you want EdgeSVG to make the best choice automatically.

**How it works:**
1. Analyzes image features (edge density, color count, transparency, aspect ratio)
2. Classifies as `logo`, `icon`, `illustration`, or `photo`
3. Routes to the best preset for that classification

```bash
edgesvg convert input.png output.svg --method auto --json
```

```python
result = edgesvg.vectorize("input.png", method="auto")
print(result["decision"])  # explains why it chose the method it used
```

---

### `logo` — Clean minimal paths

Optimized for logos, icons, and geometric shapes where path cleanliness matters.

**Best for:**
- Brand logos
- App icons
- UI element SVGs
- Monochrome designs

**Characteristics:**
- Minimal path count
- Clean curves with few nodes
- Correct path closure
- Optimized for scalability and editability

```python
result = edgesvg.vectorize("brand_logo.png", method="logo")
print(f"Path count: {result['report']['metrics']['path_count']}")  # typically < 10
```

---

### `premium` — Higher-fidelity

Multi-pass strategy for richer images with more complex shapes and colors.

**Best for:**
- Illustrations
- Multi-color designs
- Images with gradients or complex shapes
- Cases where fidelity matters more than minimal paths

**Characteristics:**
- More paths and colors
- Better SSIM on complex images
- Larger file sizes than logo mode

```python
result = edgesvg.vectorize("illustration.png", method="premium")
print(f"SSIM: {result['report']['metrics']['ssim']:.4f}")
```

---

### `smart` — Iterative quality search

Runs an iterative search trying different strategy and quality levels until the output meets the SSIM and file-size targets.

**Best for:**
- When you know exactly what quality level you need
- CI/CD pipelines with quality gates
- Batch processing with quality guarantees

**How it works:**
1. Set `target_ssim` and `max_file_size`
2. EdgeSVG tries progressively finer presets
3. Returns the first result that hits both thresholds
4. Falls back to the best result if the budget is exhausted

```python
result = edgesvg.vectorize("input.png", method="smart")
assert result["report"]["metrics"]["ssim"] >= 0.995
```

```bash
# CLI uses default smart targets
edgesvg convert input.png output.svg --method smart --json
```

---

### `convert` — Default

The standard single-pass conversion. Default if no method is specified.

---

## Mode Comparison

| Mode | Speed | SSIM | Paths | Best for |
|------|-------|------|-------|---------|
| `auto` | Fast | Good | Varies | Unknown images |
| `logo` | Fastest | High (for logos) | Very few | Logos, icons |
| `premium` | Moderate | Higher | More | Illustrations |
| `smart` | Slower | Configurable | Varies | Quality-critical |
| `convert` | Fast | Standard | Standard | General use |

## Checking Which Method Was Used

The `VectorizeResponse` always tells you what was requested vs. what was used:

```python
result = edgesvg.vectorize("input.png", method="auto")
print(result["requested_method"])  # "Auto"
print(result["effective_method"])  # "Logo"  ← what actually ran
print(result["fallback_from"])     # None     ← or the original method if fallen back
print(result["decision"])          # Human-readable explanation
```

This is especially useful in `auto` mode where you want to understand EdgeSVG's reasoning.
