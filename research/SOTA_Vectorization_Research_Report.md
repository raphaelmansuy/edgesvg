# State-of-the-Art Raster-to-SVG Vectorization Research Report

**Date**: June 2025  
**Purpose**: Comprehensive analysis of SOTA algorithms, libraries, and approaches for implementing a world-class raster-to-SVG vectorization tool with clean output and high fidelity.

---

## Executive Summary

This report presents an in-depth analysis of the current state-of-the-art in raster image to SVG vectorization. The research covers academic papers, open-source libraries, commercial tools, and emerging AI/ML approaches. Key findings indicate that modern SOTA vectorization combines:

1. **Semantic segmentation** (SAM, SAM2, SAM3) for intelligent region detection
2. **Differentiable rendering** (DiffVG) for gradient-based optimization
3. **Layer-wise approaches** (LIVE, Layered Vectorization) for structured output
4. **Deep learning priors** (diffusion models, CLIP) for semantic understanding
5. **Post-processing optimization** (SVGO) for clean, minimal SVG output

The most promising direction for Vectalab is to integrate DiffVG-based optimization with SAM segmentation, adopting a layered vectorization approach while using SVGO for output cleaning.

---

## Table of Contents

1. [Classical Algorithms](#1-classical-algorithms)
2. [Modern Deep Learning Approaches](#2-modern-deep-learning-approaches)
3. [Open-Source Libraries](#3-open-source-libraries)
4. [Commercial Competition Analysis](#4-commercial-competition-analysis)
5. [SVG Optimization Techniques](#5-svg-optimization-techniques)
6. [Segmentation Models](#6-segmentation-models)
7. [Comparative Analysis](#7-comparative-analysis)
8. [Recommendations for Vectalab](#8-recommendations-for-vectalab)
9. [Implementation Roadmap](#9-implementation-roadmap)
10. [References](#10-references)

---

## 1. Classical Algorithms

### 1.1 Potrace (Polygon Tracer)

**Source**: SourceForge (potrace.sourceforge.net)  
**Algorithm**: O(n²) where n = number of pixels

**How it works:**
1. Convert image to binary (black/white)
2. Decompose into paths using boundary tracing
3. Apply polygon approximation
4. Fit Bézier curves using least-squares optimization
5. Optimize curves with corner detection

**Key Parameters:**
- `turdsize`: Minimum path area to trace
- `alphamax`: Corner threshold (0-4/3)
- `opticurve`: Enable curve optimization
- `opttolerance`: Curve optimization tolerance

**Strengths:**
- Fast for simple images
- Produces clean, minimal paths
- Well-tested and stable
- Good for logos and simple graphics

**Weaknesses:**
- Only handles binary (black/white) images
- Requires color quantization for multi-color images
- No semantic understanding
- O(n²) complexity limits scalability

**Current Vectalab Usage**: Potrace is used in `vectalab/tracing.py` for path extraction after SAM segmentation.

### 1.2 Douglas-Peucker Algorithm

**Purpose**: Polyline simplification  
**Complexity**: O(n²) worst case, O(n log n) average

**How it works:**
1. Start with a polyline (sequence of points)
2. Find point farthest from line between endpoints
3. If distance > tolerance, recursively simplify sub-polylines
4. Else, remove intermediate points

**Libraries implementing this:**
- **simplify-js** (JavaScript): 2.4k GitHub stars, high-performance implementation
- **simplify.py** (Python): Port of simplify-js

**Use in SVG**: Essential for reducing path complexity while maintaining visual fidelity.

### 1.3 Bézier Curve Fitting

**Schneider's Algorithm**: The standard approach for fitting cubic Bézier curves to point sequences.

**Steps:**
1. Estimate tangent vectors at endpoints
2. Compute Bézier control points using least-squares
3. Calculate maximum error
4. If error > tolerance, split curve and recurse

**Key insight**: Bézier curve fitting quality directly impacts SVG file size and visual smoothness.

---

## 2. Modern Deep Learning Approaches

### 2.1 DiffVG - Differentiable Vector Graphics Rasterization

**Paper**: "Differentiable Vector Graphics Rasterization for Editing and Learning" (Li et al., 2020)  
**Repository**: github.com/BachiLi/diffvg (1.2k stars)  
**Framework**: PyTorch/TensorFlow

**Core Innovation:**
- First differentiable rasterizer for SVG primitives
- Enables gradient-based optimization of vector graphics
- Bridges the gap between vector and raster domains

**Supported Primitives:**
- Paths (Bézier curves)
- Circles, Ellipses
- Rectangles
- Polygons

**Technical Details:**
```
Forward pass: SVG → Rasterized Image
Backward pass: ∂Loss/∂(SVG params) computed via automatic differentiation
```

**Anti-aliasing**: Uses prefiltered rendering for smooth gradients through edges.

**Applications:**
- Image vectorization via optimization
- Text-to-SVG generation
- SVG editing with gradient feedback

**Integration with Vectalab**: DiffVG could replace or augment the Bayesian refinement step for higher fidelity results.

### 2.2 LIVE - Layer-wise Image Vectorization (CVPR 2022 Oral)

**Paper**: "Towards Layer-wise Image Vectorization" (Ma et al., 2022)  
**Repository**: github.com/Picsart-AI-Research/LIVE-Layerwise-Image-Vectorization  
**Achievement**: CVPR 2022 Oral presentation

**Architecture:**
```
Input Image → Path Initialization → [DiffVG Rendering → Loss Computation → Path Optimization] × N layers
```

**Key Components:**

1. **UDF Loss (Unsigned Distance Field)**:
   - Measures distance from rendered edges to target edges
   - More robust than pixel-wise L2 loss

2. **Xing Loss**:
   - Penalizes self-intersecting paths
   - Encourages cleaner, simpler curves

3. **Layered Decomposition**:
   - Processes image in layers (background to foreground)
   - Each layer adds detail progressively

**Results:**
- Achieves 19.9% compactness improvement over baselines
- Produces editable, layered SVG output

**Limitations:**
- Computationally expensive (optimization-based)
- Struggles with complex textures
- Fixed number of paths may be limiting

### 2.3 VectorFusion - Text-to-SVG (CVPR 2023)

**Paper**: "VectorFusion: Text-to-SVG by Abstracting Pixel-Based Diffusion Models" (Jain et al., 2023)

**Approach:**
1. Use Stable Diffusion as image prior
2. Apply Score Distillation Sampling (SDS) loss
3. Optimize SVG parameters to match diffusion model's understanding

**SDS Loss Formula:**
```
∇_θ L_SDS = E_t,ε[w(t)(ε_φ(z_t, y, t) - ε)∂x/∂θ]
```
Where:
- ε_φ: Noise predicted by diffusion model
- y: Text prompt
- θ: SVG parameters

**Key Insight**: Leverages pre-trained diffusion models without retraining for SVG domain.

**Limitations:**
- Very slow (10+ minutes per image)
- Produces intertwined, hard-to-edit paths
- Works best for simple icons/logos

### 2.4 SVGDreamer - Enhanced Text-to-SVG

**Paper**: "SVGDreamer: Text Guided SVG Generation with Diffusion Model" (Xing et al., 2024)

**Improvements over VectorFusion:**
- Variational Score Distillation (VSD) for higher quality
- Semantic component-level optimization
- Better handling of complex scenes

**Architecture:**
```
Text → [SDXL Prior] → VSD Optimization → Semantic Decomposition → SVG
```

### 2.5 Layered Image Vectorization via Semantic Simplification (CVPR 2024)

**Paper**: arXiv:2406.05404v1  
**Key Innovation**: Combines SDS with SAM segmentation

**Architecture:**
```
Input Image → SAM Segmentation → Semantic Simplification → SDS Optimization → Layered SVG
```

**Performance:**
- **43.9% compactness** (vs 19.9% for LIVE)
- Better semantic coherence
- More editable output

**Key Techniques:**
1. **Semantic Mask Simplification**: Uses SAM to identify semantic regions, then simplifies boundaries
2. **Bayesian Inversion**: Incorporates prior knowledge about natural image structure
3. **Layer-wise Construction**: Builds SVG from back to front

### 2.6 SVGFusion - Scalable Text-to-SVG (December 2024)

**Paper**: arXiv:2412.10437v2  
**Innovation**: Latent diffusion for SVG generation

**Architecture:**
1. **VP-VAE (Vector-Pixel Fusion VAE)**: Learns latent space from both SVG code and rasterization
2. **VS-DiT (Vector Space Diffusion Transformer)**: Generates in learned latent space

**Key Features:**
- **Rendering Sequence Modeling**: Learns human design logic (order of SVG element creation)
- **Scalable**: Can add more DiT blocks for better quality
- **Editable Output**: Clean layer structure

**Dataset**: SVGX-Dataset (240k high-quality human-designed SVGs)

**Results (Table 1 from paper):**
| Method | FID↓ | CLIPScore↑ | Time↓ |
|--------|------|------------|-------|
| VectorFusion | 84.53 | 0.309 | 10min |
| SVGDreamer | 70.10 | 0.360 | 35min |
| SVGFusion-L | **35.45** | **0.403** | **1.2s** |

**Key Insight**: SVGFusion is 500x faster than optimization-based methods while producing higher quality results.

### 2.7 SAMVG - SAM-based Vectorization

**Paper**: arXiv:2311.05276

**Approach:**
1. Apply SAM with multiple prompts (grid of points)
2. Filter masks to select best dense segmentation
3. Convert masks to vector paths
4. Optimize paths for smoothness

**Advantage**: Leverages SAM's zero-shot segmentation capability for arbitrary images.

---

## 3. Open-Source Libraries

### 3.1 vtracer

**Repository**: github.com/nickt28/vtracer  
**Stars**: 5.1k  
**Language**: Rust with Python bindings

**Algorithm**: O(n) complexity (vs Potrace's O(n²))

**Key Features:**
- Handles color images directly (no color quantization needed)
- Stacked mode: Layers colors from bottom to top
- Cutout mode: Subtracts overlapping regions
- Hierarchical clustering for color reduction

**Parameters:**
```python
vtracer.convert_image_to_svg_py(
    image_path,
    colormode='color',  # 'color' or 'binary'
    mode='stacked',     # 'stacked' or 'cutout'
    filter_speckle=4,   # Remove small artifacts
    color_precision=6,  # Bits per color channel
    layer_difference=16,# Color grouping threshold
    corner_threshold=60,# Curve vs corner decision
    length_threshold=4, # Minimum path length
    max_iterations=10,  # Optimization iterations
    splice_threshold=45,# Path joining threshold
    path_precision=3    # Coordinate decimal places
)
```

**Strengths:**
- Very fast (O(n) vs O(n²))
- Good quality for most images
- Active development
- Python bindings available

**Current Vectalab Usage**: Used in `vectalab/hifi.py` for high-fidelity vectorization.

### 3.2 Potrace (libpotrace)

**Website**: potrace.sourceforge.net  
**Bindings**: pypotrace (Python)

**Current Vectalab Usage**: Used in `vectalab/tracing.py` for bitmap tracing.

### 3.3 SVGO - SVG Optimizer

**Repository**: github.com/svg/svgo  
**Stars**: 22.1k  
**Dependents**: 17M+ npm packages

**Purpose**: Post-processing optimization of SVG files

**Key Plugins:**
| Plugin | Description |
|--------|-------------|
| `removeDoctype` | Remove DOCTYPE declaration |
| `removeXMLProcInst` | Remove XML processing instructions |
| `removeComments` | Remove comments |
| `removeMetadata` | Remove `<metadata>` |
| `removeTitle` | Remove `<title>` |
| `removeDesc` | Remove `<desc>` |
| `removeUselessDefs` | Remove unused `<defs>` |
| `removeEmptyAttrs` | Remove empty attributes |
| `removeHiddenElems` | Remove hidden elements |
| `removeEmptyContainers` | Remove empty containers |
| `cleanupEnableBackground` | Remove deprecated enable-background |
| `convertStyleToAttrs` | Convert styles to attributes |
| `convertColors` | Optimize color representations |
| `convertPathData` | Optimize path data |
| `convertTransform` | Optimize transformations |
| `removeUnusedNS` | Remove unused namespaces |
| `mergePaths` | Merge adjacent paths |
| `collapseGroups` | Collapse unnecessary groups |
| `removeRasterImages` | Remove embedded raster images |
| `sortAttrs` | Sort attributes for consistency |
| `sortDefsChildren` | Sort defs children |
| `removeDimensions` | Remove width/height, use viewBox |

**Usage:**
```bash
npx svgo input.svg -o output.svg
```

**Python Alternative**: `scour` (less powerful but pure Python)

### 3.4 Primitive

**Repository**: github.com/fogleman/primitive  
**Language**: Go

**Approach**: Reconstructs images using geometric primitives

**Supported Shapes:**
- Triangles
- Rectangles
- Ellipses
- Circles
- Polygons
- Bézier curves

**Algorithm:**
1. Start with average color background
2. Iteratively add shapes that minimize error
3. Use hill-climbing optimization for shape parameters

**Use Case**: Artistic/stylized vectorization rather than faithful reproduction.

### 3.5 ImageTracer.js

**Repository**: github.com/nickt28/imagetracerjs

**Features:**
- Pure JavaScript implementation
- Browser-compatible
- Multiple color modes
- Adjustable quality/speed tradeoff

---

## 4. Commercial Competition Analysis

### 4.1 Vectorizer.ai

**Website**: vectorizer.ai  
**Technology**: Deep learning + classical algorithms

**Claimed Features:**
1. **Full Shape Fitting**: Detects circles, ellipses, rounded rectangles (not just Bézier curves)
2. **Sub-pixel Precision**: Analyzes at higher resolution than input
3. **Accurate Corners**: Properly distinguishes corners from curves
4. **Tangent Matching**: Smooth transitions between shapes
5. **Color Accuracy**: High-fidelity color reproduction
6. **Symmetry Modeling**: Detects and preserves symmetry

**Technical Approach:**
- Trains deep neural networks on image-to-vector pairs
- Uses both supervised and unsupervised learning
- Post-processes with classical optimization

**Quality Indicators:**
- Handles complex photographic images
- Preserves fine details
- Produces clean, editable output

### 4.2 Vector Magic (Desktop Edition)

**Background**: Based on 2008 Stanford research by Jonathan Diebel

**Algorithm**: Bayesian Inversion (as described in docs/algorithm.md)

**Key Innovation**: 
- Probabilistic approach to vectorization
- Models natural image statistics
- Iterative refinement process

**Process:**
1. Color quantization with user guidance
2. Region segmentation
3. Boundary refinement via Bayesian optimization
4. Bézier curve fitting
5. Quality optimization

**Current Vectalab Implementation**: `vectalab/bayesian.py` implements a similar approach.

### 4.3 Adobe Illustrator Image Trace

**Features:**
- Multiple tracing modes (high fidelity, 3-color, 16-color, etc.)
- Path smoothing controls
- Corner angle threshold
- Noise reduction

**Approach**: Likely uses a combination of:
- Color quantization
- Edge detection
- Potrace-style tracing
- Proprietary smoothing algorithms

---

## 5. SVG Optimization Techniques

### 5.1 Path Optimization

**Coordinate Precision:**
```xml
<!-- Before -->
<path d="M 12.345678 23.456789 L 45.678901 67.890123"/>

<!-- After (precision=2) -->
<path d="M12.35 23.46L45.68 67.89"/>
```

**Path Command Optimization:**
```xml
<!-- Before -->
<path d="M 0 0 L 10 0 L 10 10 L 0 10 Z"/>

<!-- After -->
<path d="M0 0h10v10H0z"/>
```

### 5.2 Color Optimization

```xml
<!-- Before -->
<rect fill="#ffffff"/>
<rect fill="rgb(255, 0, 0)"/>

<!-- After -->
<rect fill="#fff"/>
<rect fill="red"/>
```

### 5.3 Transform Optimization

```xml
<!-- Before -->
<g transform="translate(10, 0) translate(0, 20) rotate(45) rotate(-45)">

<!-- After -->
<g transform="translate(10 20)">
```

### 5.4 Structure Optimization

**Merge Adjacent Paths:**
```xml
<!-- Before -->
<path d="M0 0L10 10" fill="red"/>
<path d="M10 10L20 20" fill="red"/>

<!-- After -->
<path d="M0 0L10 10L20 20" fill="red"/>
```

**Collapse Groups:**
```xml
<!-- Before -->
<g><g><g><rect/></g></g></g>

<!-- After -->
<rect/>
```

### 5.5 Python SVGO Wrapper

```python
import subprocess
import json

def optimize_svg(input_path: str, output_path: str, config: dict = None) -> None:
    """Optimize SVG using SVGO."""
    cmd = ['npx', 'svgo', input_path, '-o', output_path]
    
    if config:
        config_json = json.dumps(config)
        cmd.extend(['--config', config_json])
    
    subprocess.run(cmd, check=True)
```

---

## 6. Segmentation Models

### 6.1 SAM (Segment Anything Model)

**Paper**: "Segment Anything" (Kirillov et al., 2023)  
**Repository**: github.com/facebookresearch/segment-anything

**Model Sizes:**
| Model | Parameters | Speed | Quality |
|-------|------------|-------|---------|
| ViT-B | 91M | Fast | Good |
| ViT-L | 308M | Medium | Better |
| ViT-H | 636M | Slow | Best |

**Current Vectalab Usage**: Uses SAM for semantic segmentation in `vectalab/segmentation.py`.

### 6.2 SAM 2 (2024)

**Improvements:**
- Video segmentation support
- Faster inference
- Better small object detection
- Memory-efficient architecture

**Architecture:**
```
Image → Image Encoder → Memory Attention → Mask Decoder → Segmentation
                              ↑
                        Memory Bank
```

### 6.3 SAM 3 (2025)

**New Capability**: Promptable concept segmentation

**Features:**
- Text-based segmentation prompts
- Concept understanding (e.g., "all furniture")
- Part-level segmentation
- Improved edge precision

**Relevance for Vectalab**: SAM 3 could enable semantic-aware vectorization (e.g., "vectorize the car separately from the background").

### 6.4 MobileSAM

**Purpose**: Lightweight SAM for edge devices

**Trade-offs:**
- 60x faster than SAM-H
- Slightly lower quality
- 5% of parameters

### 6.5 FastSAM

**Approach**: CNN-based (vs transformer)

**Speed**: 50x faster than SAM

**Quality**: Comparable for simple scenes, worse for complex images

---

## 7. Comparative Analysis

### 7.1 Algorithm Comparison

| Algorithm | Complexity | Color Support | Semantic | Editability | Quality |
|-----------|------------|---------------|----------|-------------|---------|
| Potrace | O(n²) | Binary only | No | High | Good |
| vtracer | O(n) | Full color | No | High | Good |
| LIVE | O(iter) | Full color | Partial | High | Very Good |
| VectorFusion | O(iter) | Full color | Yes | Low | Good |
| SVGDreamer | O(iter) | Full color | Yes | Medium | Very Good |
| Layered Vec. | O(iter) | Full color | Yes | High | Excellent |
| SVGFusion | O(1) | Full color | Yes | High | Excellent |

### 7.2 Library Comparison

| Library | Stars | Language | Speed | Quality | Maintained |
|---------|-------|----------|-------|---------|------------|
| vtracer | 5.1k | Rust | Very Fast | Good | Yes |
| Potrace | N/A | C | Fast | Good | Stable |
| DiffVG | 1.2k | C++/Python | Slow | Excellent | Moderate |
| SVGO | 22.1k | JavaScript | Fast | N/A | Yes |
| simplify-js | 2.4k | JavaScript | Very Fast | N/A | Stable |

### 7.3 Quality Metrics

**SSIM (Structural Similarity Index):**
- Current Vectalab target: 99.8%
- Industry standard: >95%
- Photorealistic: >99%

**File Size Efficiency:**
- Optimal: <10KB for simple logos
- Good: <50KB for icons
- Acceptable: <200KB for complex illustrations

**Path Complexity:**
- Optimal: <100 paths for simple images
- Good: <500 paths for medium complexity
- Acceptable: <2000 paths for detailed images

---

## 8. Recommendations for Vectalab

### 8.1 Short-term Improvements (1-2 months)

#### 8.1.1 Integrate SVGO Post-processing
```python
# Add to vectalab/output.py
def optimize_svg_output(svg_path: str) -> str:
    """Post-process SVG with SVGO for cleaner output."""
    optimized_path = svg_path.replace('.svg', '_optimized.svg')
    subprocess.run([
        'npx', 'svgo', svg_path, '-o', optimized_path,
        '--multipass',
        '--precision=2'
    ], check=True)
    return optimized_path
```

#### 8.1.2 Upgrade to SAM 2
- Better edge precision
- Faster inference
- Improved small object detection

#### 8.1.3 Add Path Simplification
```python
# Use Douglas-Peucker for path simplification
from simplify import simplify

def simplify_path(path_points: list, tolerance: float = 1.0) -> list:
    """Reduce path complexity while maintaining shape."""
    return simplify(path_points, tolerance, highQuality=True)
```

### 8.2 Medium-term Improvements (3-6 months)

#### 8.2.1 Implement DiffVG Integration
```python
# Add differentiable rendering capability
import diffvg

def optimize_svg_with_diffvg(
    target_image: np.ndarray,
    initial_svg: str,
    num_iterations: int = 500
) -> str:
    """Use DiffVG for gradient-based SVG refinement."""
    # Parse initial SVG
    shapes, shape_groups = parse_svg(initial_svg)
    
    # Optimization loop
    optimizer = torch.optim.Adam(get_svg_params(shapes), lr=0.1)
    for i in range(num_iterations):
        rendered = diffvg.render(shapes, shape_groups)
        loss = compute_loss(rendered, target_image)
        loss.backward()
        optimizer.step()
        optimizer.zero_grad()
    
    return export_svg(shapes, shape_groups)
```

#### 8.2.2 Layer-wise Vectorization
Implement LIVE-style layered approach:
1. Segment image into semantic layers
2. Vectorize each layer separately
3. Compose layers in depth order

#### 8.2.3 Shape Primitive Detection
Beyond Bézier curves, detect and fit:
- Circles
- Ellipses
- Rectangles (including rounded)
- Regular polygons

### 8.3 Long-term Vision (6-12 months)

#### 8.3.1 Neural Path Generation
Train a model similar to SVGFusion's VP-VAE:
- Learn latent space from high-quality SVGs
- Generate paths that follow human design conventions

#### 8.3.2 Text-guided Vectorization
Leverage SAM 3's concept understanding:
```python
def semantic_vectorize(image: np.ndarray, prompt: str) -> str:
    """Vectorize with semantic guidance."""
    # "Focus on the foreground object"
    # "Keep fine details in the face region"
    # "Simplify the background"
```

#### 8.3.3 Multi-scale Processing
```
High-resolution details (fine edges, textures)
         ↓
Mid-resolution structure (shapes, boundaries)
         ↓
Low-resolution semantics (regions, layers)
```

---

## 9. Implementation Roadmap

### Phase 1: Foundation (Month 1-2)

```markdown
- [ ] Integrate SVGO for SVG optimization
- [ ] Upgrade from SAM to SAM 2
- [ ] Add path simplification (Douglas-Peucker)
- [ ] Implement coordinate precision control
- [ ] Add color quantization improvements
```

### Phase 2: Quality Enhancement (Month 3-4)

```markdown
- [ ] Integrate DiffVG for gradient-based refinement
- [ ] Implement UDF loss (from LIVE)
- [ ] Add Xing loss for path simplicity
- [ ] Implement shape primitive detection (circles, rectangles)
- [ ] Add symmetry detection
```

### Phase 3: Advanced Features (Month 5-6)

```markdown
- [ ] Implement layered vectorization (LIVE approach)
- [ ] Add semantic-aware path grouping
- [ ] Integrate SAM 3 for concept-level control
- [ ] Implement adaptive quality settings
- [ ] Add batch processing optimization
```

### Phase 4: Neural Approach (Month 7-12)

```markdown
- [ ] Collect/curate high-quality SVG dataset
- [ ] Train VP-VAE for SVG latent space
- [ ] Implement diffusion-based path generation
- [ ] Add text-guided vectorization
- [ ] Implement multi-scale processing
```

---

## 10. References

### Academic Papers

1. Li, T. M., et al. (2020). "Differentiable Vector Graphics Rasterization for Editing and Learning." ACM TOG.

2. Ma, X., et al. (2022). "Towards Layer-wise Image Vectorization." CVPR 2022 (Oral).

3. Jain, A., et al. (2023). "VectorFusion: Text-to-SVG by Abstracting Pixel-Based Diffusion Models." CVPR 2023.

4. Xing, X., et al. (2024). "SVGDreamer: Text Guided SVG Generation with Diffusion Model." CVPR 2024.

5. Xing, X., et al. (2024). "Layered Image Vectorization via Semantic Simplification." arXiv:2406.05404.

6. Xing, X., et al. (2024). "SVGFusion: Scalable Text-to-SVG Generation via Vector Space Diffusion." arXiv:2412.10437.

7. Kirillov, A., et al. (2023). "Segment Anything." ICCV 2023.

8. Diebel, J., et al. (2008). "Bayesian Image Vectorization." Stanford University Thesis.

### Libraries and Tools

1. **vtracer**: github.com/nickt28/vtracer
2. **DiffVG**: github.com/BachiLi/diffvg
3. **LIVE**: github.com/Picsart-AI-Research/LIVE-Layerwise-Image-Vectorization
4. **SVGO**: github.com/svg/svgo
5. **Potrace**: potrace.sourceforge.net
6. **simplify-js**: github.com/mourner/simplify-js
7. **SAM**: github.com/facebookresearch/segment-anything

### Commercial Tools

1. **Vectorizer.ai**: vectorizer.ai
2. **Vector Magic**: vectormagic.com
3. **Adobe Illustrator Image Trace**: adobe.com/products/illustrator

---

## Appendix A: Key Metrics and Benchmarks

### A.1 Quality Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| SSIM | Structural similarity | >0.95 |
| PSNR | Peak signal-to-noise ratio | >30 dB |
| FID | Fréchet Inception Distance | <50 |
| Path Count | Number of SVG paths | Minimize |
| File Size | Compressed SVG size | <50KB for icons |

### A.2 Speed Benchmarks (1024x1024 image)

| Method | Time | Hardware |
|--------|------|----------|
| vtracer | <1s | CPU |
| Potrace | 1-2s | CPU |
| SAM + Potrace | 2-5s | GPU |
| DiffVG optimization | 30-60s | GPU |
| LIVE | 5-10min | GPU |
| VectorFusion | 10-30min | GPU |
| SVGFusion | 1-2s | GPU |

---

## Appendix B: Code Examples

### B.1 Complete DiffVG Integration Example

```python
import torch
import diffvg
import numpy as np
from PIL import Image

def optimize_paths_diffvg(
    target: np.ndarray,
    initial_paths: list,
    num_iterations: int = 500,
    learning_rate: float = 0.1
) -> list:
    """
    Optimize SVG paths using DiffVG's differentiable rendering.
    
    Args:
        target: Target image as numpy array (H, W, 3)
        initial_paths: List of initial path definitions
        num_iterations: Number of optimization iterations
        learning_rate: Adam optimizer learning rate
    
    Returns:
        Optimized path definitions
    """
    device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
    
    # Convert target to tensor
    target_tensor = torch.from_numpy(target).float().to(device) / 255.0
    
    # Initialize paths as differentiable tensors
    shapes = []
    shape_groups = []
    for path_def in initial_paths:
        points = torch.tensor(path_def['points'], requires_grad=True, device=device)
        color = torch.tensor(path_def['color'], requires_grad=True, device=device)
        shapes.append(diffvg.Path(points=points, ...))
        shape_groups.append(diffvg.ShapeGroup(shape_ids=[len(shapes)-1], fill_color=color))
    
    # Optimizer
    params = [p for s in shapes for p in s.parameters()] + \
             [g.fill_color for g in shape_groups]
    optimizer = torch.optim.Adam(params, lr=learning_rate)
    
    # Optimization loop
    for i in range(num_iterations):
        optimizer.zero_grad()
        
        # Render current paths
        scene = diffvg.Scene(shapes, shape_groups)
        rendered = diffvg.render(scene, width=target.shape[1], height=target.shape[0])
        
        # Compute loss
        loss = torch.nn.functional.mse_loss(rendered, target_tensor)
        
        # Backprop and update
        loss.backward()
        optimizer.step()
        
        if i % 100 == 0:
            print(f"Iteration {i}, Loss: {loss.item():.6f}")
    
    # Extract optimized paths
    return [extract_path_def(s, g) for s, g in zip(shapes, shape_groups)]
```

### B.2 SVGO Integration

```python
import subprocess
import tempfile
import os

def optimize_svg_with_svgo(svg_content: str, config: dict = None) -> str:
    """
    Optimize SVG content using SVGO.
    
    Args:
        svg_content: Input SVG as string
        config: Optional SVGO configuration
    
    Returns:
        Optimized SVG content
    """
    default_config = {
        "multipass": True,
        "plugins": [
            "preset-default",
            {"name": "removeViewBox", "active": False},
            {"name": "cleanupNumericValues", "params": {"floatPrecision": 2}},
            {"name": "mergePaths", "active": True},
            {"name": "collapseGroups", "active": True},
        ]
    }
    
    config = config or default_config
    
    with tempfile.NamedTemporaryFile(mode='w', suffix='.svg', delete=False) as f_in:
        f_in.write(svg_content)
        input_path = f_in.name
    
    output_path = input_path.replace('.svg', '_opt.svg')
    
    try:
        cmd = ['npx', 'svgo', input_path, '-o', output_path]
        subprocess.run(cmd, check=True, capture_output=True)
        
        with open(output_path, 'r') as f:
            return f.read()
    finally:
        os.unlink(input_path)
        if os.path.exists(output_path):
            os.unlink(output_path)
```

---

*Report prepared for Vectalab project enhancement*
*Last updated: June 2025*
