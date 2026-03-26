# Technical Deep-Dive: Implementing SOTA Vectorization

This document provides implementation-specific details for integrating state-of-the-art vectorization techniques into Vectalab.

---

## 1. DiffVG Integration

### 1.1 Installation

```bash
# Clone DiffVG
git clone https://github.com/BachiLi/diffvg.git
cd diffvg

# Install dependencies
pip install pybind11

# Build and install
python setup.py install
```

### 1.2 Core Concepts

DiffVG represents SVG elements as differentiable tensors:

```python
import diffvg
import torch

# Define a simple path
num_control_points = torch.tensor([2])  # Number of control points per segment
points = torch.tensor([
    [100.0, 100.0],  # Start point
    [150.0, 50.0],   # Control point 1
    [200.0, 100.0],  # Control point 2
    [200.0, 150.0],  # End point
], requires_grad=True)

path = diffvg.Path(
    num_control_points=num_control_points,
    points=points,
    is_closed=True,
    stroke_width=torch.tensor(0.0)
)

# Define fill color
fill_color = torch.tensor([1.0, 0.0, 0.0, 1.0], requires_grad=True)  # RGBA

shape_group = diffvg.ShapeGroup(
    shape_ids=torch.tensor([0]),
    fill_color=fill_color
)
```

### 1.3 Rendering Pipeline

```python
def render_svg(shapes, shape_groups, width, height):
    """Render SVG shapes to image using DiffVG."""
    scene = diffvg.RenderFunction.apply(
        width,
        height,
        2,  # num_samples_x for anti-aliasing
        2,  # num_samples_y for anti-aliasing
        0,  # seed
        None,  # background
        *shapes_to_args(shapes, shape_groups)
    )
    return scene

def shapes_to_args(shapes, shape_groups):
    """Convert shapes and groups to DiffVG render arguments."""
    args = []
    args.append(len(shapes))  # Number of shapes
    
    for shape in shapes:
        # Append shape type and parameters
        if isinstance(shape, diffvg.Path):
            args.extend([0, shape.num_control_points, shape.points, ...])
        elif isinstance(shape, diffvg.Circle):
            args.extend([1, shape.center, shape.radius, ...])
        # ... other shape types
    
    args.append(len(shape_groups))
    for group in shape_groups:
        args.extend([group.shape_ids, group.fill_color, ...])
    
    return args
```

### 1.4 Loss Functions

#### Pixel-wise L2 Loss
```python
def pixel_loss(rendered, target):
    """Standard L2 loss between rendered and target images."""
    return torch.mean((rendered - target) ** 2)
```

#### Perceptual Loss (using VGG)
```python
import torchvision.models as models

class PerceptualLoss(torch.nn.Module):
    def __init__(self):
        super().__init__()
        vgg = models.vgg16(pretrained=True).features[:16]
        self.vgg = vgg.eval()
        for param in self.vgg.parameters():
            param.requires_grad = False
    
    def forward(self, rendered, target):
        # Normalize to VGG input range
        rendered_norm = self.normalize(rendered)
        target_norm = self.normalize(target)
        
        # Extract features
        rendered_features = self.vgg(rendered_norm)
        target_features = self.vgg(target_norm)
        
        return torch.nn.functional.mse_loss(rendered_features, target_features)
    
    def normalize(self, x):
        mean = torch.tensor([0.485, 0.456, 0.406]).view(1, 3, 1, 1)
        std = torch.tensor([0.229, 0.224, 0.225]).view(1, 3, 1, 1)
        return (x - mean) / std
```

#### UDF Loss (Unsigned Distance Field) - from LIVE
```python
def udf_loss(rendered_edges, target_edges, sigma=3.0):
    """
    Unsigned Distance Field loss for edge alignment.
    
    More robust than pixel-wise loss for vector graphics.
    """
    # Compute distance transform of target edges
    target_udf = compute_distance_transform(target_edges)
    
    # Compute loss as weighted sum of distances
    loss = torch.sum(rendered_edges * target_udf) / (torch.sum(rendered_edges) + 1e-8)
    
    return loss

def compute_distance_transform(edges):
    """Compute unsigned distance field from edge map."""
    from scipy import ndimage
    edges_np = edges.detach().cpu().numpy()
    udf = ndimage.distance_transform_edt(1 - edges_np)
    return torch.from_numpy(udf).to(edges.device)
```

#### Xing Loss (Self-intersection Penalty) - from LIVE
```python
def xing_loss(paths):
    """
    Penalize self-intersecting paths.
    
    Encourages cleaner, simpler curves.
    """
    total_loss = 0.0
    
    for path in paths:
        points = path.points.view(-1, 2)
        n_points = points.shape[0]
        
        # Check each pair of non-adjacent segments
        for i in range(n_points - 3):
            for j in range(i + 2, n_points - 1):
                p1, p2 = points[i], points[i + 1]
                p3, p4 = points[j], points[j + 1]
                
                # Compute intersection
                if segments_intersect(p1, p2, p3, p4):
                    # Add soft penalty
                    total_loss += intersection_penalty(p1, p2, p3, p4)
    
    return total_loss

def segments_intersect(p1, p2, p3, p4):
    """Check if line segments (p1,p2) and (p3,p4) intersect."""
    d1 = direction(p3, p4, p1)
    d2 = direction(p3, p4, p2)
    d3 = direction(p1, p2, p3)
    d4 = direction(p1, p2, p4)
    
    if ((d1 > 0 and d2 < 0) or (d1 < 0 and d2 > 0)) and \
       ((d3 > 0 and d4 < 0) or (d3 < 0 and d4 > 0)):
        return True
    return False

def direction(p1, p2, p3):
    """Compute cross product for orientation."""
    return (p3[0] - p1[0]) * (p2[1] - p1[1]) - (p2[0] - p1[0]) * (p3[1] - p1[1])
```

---

## 2. Layered Vectorization (LIVE Approach)

### 2.1 Architecture

```
Input Image
    ↓
[Semantic Segmentation (SAM)]
    ↓
Layer Ordering (depth estimation)
    ↓
For each layer (back to front):
    ├── Initialize paths
    ├── DiffVG optimization
    ├── Path simplification
    └── Add to scene
    ↓
Compose final SVG
```

### 2.2 Implementation

```python
class LayeredVectorizer:
    def __init__(self, sam_model, diffvg_config):
        self.sam = sam_model
        self.config = diffvg_config
        
    def vectorize(self, image: np.ndarray) -> str:
        """
        Perform layer-wise vectorization.
        
        Args:
            image: Input RGB image (H, W, 3)
        
        Returns:
            SVG string
        """
        # Step 1: Segment image
        masks = self.segment_image(image)
        
        # Step 2: Order layers by depth
        ordered_masks = self.order_by_depth(masks, image)
        
        # Step 3: Vectorize each layer
        layers = []
        composite = np.zeros_like(image)
        
        for i, mask in enumerate(ordered_masks):
            # Extract layer region
            layer_image = image * mask[:, :, np.newaxis]
            
            # Compute residual (what's not yet represented)
            residual = np.abs(layer_image - composite)
            
            # Initialize paths for this layer
            paths = self.initialize_paths(mask, layer_image)
            
            # Optimize paths with DiffVG
            optimized_paths = self.optimize_layer(
                paths, layer_image, residual
            )
            
            layers.append({
                'paths': optimized_paths,
                'z_order': i
            })
            
            # Update composite
            composite = self.render_composite(layers)
        
        # Step 4: Compose SVG
        return self.compose_svg(layers, image.shape)
    
    def segment_image(self, image):
        """Segment image using SAM."""
        # Generate automatic masks
        mask_generator = SamAutomaticMaskGenerator(self.sam)
        masks = mask_generator.generate(image)
        
        # Sort by area (larger first for background)
        masks.sort(key=lambda x: x['area'], reverse=True)
        
        return [m['segmentation'] for m in masks]
    
    def order_by_depth(self, masks, image):
        """Order masks from background to foreground."""
        # Simple heuristic: larger masks are background
        # Could use monocular depth estimation for better results
        return masks  # Already sorted by area
    
    def initialize_paths(self, mask, layer_image):
        """Initialize paths from mask contours."""
        import cv2
        
        # Find contours
        contours, _ = cv2.findContours(
            mask.astype(np.uint8),
            cv2.RETR_EXTERNAL,
            cv2.CHAIN_APPROX_SIMPLE
        )
        
        paths = []
        for contour in contours:
            if len(contour) < 3:
                continue
            
            # Approximate contour with fewer points
            epsilon = 0.01 * cv2.arcLength(contour, True)
            approx = cv2.approxPolyDP(contour, epsilon, True)
            
            # Convert to Bézier path
            path = self.contour_to_bezier(approx)
            
            # Get average color
            color = self.get_region_color(layer_image, mask)
            
            paths.append({
                'points': path,
                'color': color
            })
        
        return paths
    
    def contour_to_bezier(self, contour):
        """Convert contour points to cubic Bézier path."""
        points = contour.squeeze()
        n = len(points)
        
        bezier_points = []
        for i in range(n):
            p0 = points[i]
            p3 = points[(i + 1) % n]
            
            # Simple cubic Bézier with control points at 1/3 and 2/3
            p1 = p0 + (p3 - p0) / 3
            p2 = p0 + 2 * (p3 - p0) / 3
            
            bezier_points.extend([p0, p1, p2])
        
        bezier_points.append(points[0])  # Close path
        
        return np.array(bezier_points, dtype=np.float32)
    
    def get_region_color(self, image, mask):
        """Get average color of masked region."""
        masked = image[mask]
        return np.mean(masked, axis=0) / 255.0
    
    def optimize_layer(self, paths, target, residual):
        """Optimize paths for a single layer using DiffVG."""
        # Convert to DiffVG format
        diffvg_paths = self.to_diffvg_paths(paths)
        
        # Setup optimizer
        params = []
        for path in diffvg_paths:
            params.append(path['points'])
            params.append(path['color'])
        
        optimizer = torch.optim.Adam(params, lr=self.config.lr)
        
        # Convert target to tensor
        target_tensor = torch.from_numpy(target).float().cuda() / 255.0
        residual_tensor = torch.from_numpy(residual).float().cuda() / 255.0
        
        # Optimization loop
        for iteration in range(self.config.num_iterations):
            optimizer.zero_grad()
            
            # Render
            rendered = self.render_diffvg(diffvg_paths)
            
            # Compute losses
            l_pixel = pixel_loss(rendered, target_tensor)
            l_udf = udf_loss(
                self.extract_edges(rendered),
                self.extract_edges(target_tensor)
            )
            l_xing = xing_loss([p['points'] for p in diffvg_paths])
            
            # Weighted sum (from LIVE paper)
            loss = l_pixel + 0.1 * l_udf + 0.01 * l_xing
            
            # Backprop
            loss.backward()
            optimizer.step()
            
            # Clamp colors to valid range
            for path in diffvg_paths:
                path['color'].data.clamp_(0, 1)
        
        return diffvg_paths
    
    def compose_svg(self, layers, image_shape):
        """Compose final SVG from layers."""
        height, width = image_shape[:2]
        
        svg_parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" '
            f'viewBox="0 0 {width} {height}">'
        ]
        
        for layer in layers:
            for path_data in layer['paths']:
                path_str = self.points_to_svg_path(path_data['points'])
                color = self.color_to_hex(path_data['color'])
                svg_parts.append(f'<path d="{path_str}" fill="{color}"/>')
        
        svg_parts.append('</svg>')
        
        return '\n'.join(svg_parts)
    
    def points_to_svg_path(self, points):
        """Convert Bézier points to SVG path d attribute."""
        if len(points) < 4:
            return ""
        
        d = f"M{points[0][0]:.2f},{points[0][1]:.2f}"
        
        i = 1
        while i < len(points) - 2:
            c1, c2, end = points[i], points[i+1], points[i+2]
            d += f" C{c1[0]:.2f},{c1[1]:.2f} {c2[0]:.2f},{c2[1]:.2f} {end[0]:.2f},{end[1]:.2f}"
            i += 3
        
        d += " Z"
        return d
    
    def color_to_hex(self, color):
        """Convert RGB array to hex color."""
        r, g, b = [int(c * 255) for c in color[:3]]
        return f"#{r:02x}{g:02x}{b:02x}"
```

---

## 3. Shape Primitive Detection

### 3.1 Circle Detection

```python
import cv2
import numpy as np

def detect_circles(mask: np.ndarray, tolerance: float = 0.1) -> list:
    """
    Detect circular regions in a binary mask.
    
    Args:
        mask: Binary mask (H, W)
        tolerance: Maximum deviation from perfect circle (0-1)
    
    Returns:
        List of detected circles: [(cx, cy, radius), ...]
    """
    # Find contours
    contours, _ = cv2.findContours(
        mask.astype(np.uint8),
        cv2.RETR_EXTERNAL,
        cv2.CHAIN_APPROX_SIMPLE
    )
    
    circles = []
    for contour in contours:
        # Fit minimum enclosing circle
        (cx, cy), radius = cv2.minEnclosingCircle(contour)
        
        # Compute area ratio
        contour_area = cv2.contourArea(contour)
        circle_area = np.pi * radius ** 2
        
        if circle_area > 0:
            ratio = contour_area / circle_area
            
            # Check if close to a circle
            if 1 - tolerance <= ratio <= 1 + tolerance:
                circles.append((cx, cy, radius))
    
    return circles


def detect_ellipses(mask: np.ndarray, tolerance: float = 0.1) -> list:
    """
    Detect elliptical regions in a binary mask.
    
    Returns:
        List of ellipses: [(cx, cy, rx, ry, angle), ...]
    """
    contours, _ = cv2.findContours(
        mask.astype(np.uint8),
        cv2.RETR_EXTERNAL,
        cv2.CHAIN_APPROX_SIMPLE
    )
    
    ellipses = []
    for contour in contours:
        if len(contour) < 5:  # Need at least 5 points for ellipse fitting
            continue
        
        # Fit ellipse
        ellipse = cv2.fitEllipse(contour)
        (cx, cy), (width, height), angle = ellipse
        
        # Compute area ratio
        contour_area = cv2.contourArea(contour)
        ellipse_area = np.pi * (width / 2) * (height / 2)
        
        if ellipse_area > 0:
            ratio = contour_area / ellipse_area
            
            if 1 - tolerance <= ratio <= 1 + tolerance:
                ellipses.append((cx, cy, width / 2, height / 2, angle))
    
    return ellipses
```

### 3.2 Rectangle Detection

```python
def detect_rectangles(mask: np.ndarray, tolerance: float = 0.1) -> list:
    """
    Detect rectangular regions in a binary mask.
    
    Returns:
        List of rectangles: [(cx, cy, width, height, angle), ...]
    """
    contours, _ = cv2.findContours(
        mask.astype(np.uint8),
        cv2.RETR_EXTERNAL,
        cv2.CHAIN_APPROX_SIMPLE
    )
    
    rectangles = []
    for contour in contours:
        # Fit minimum area rectangle
        rect = cv2.minAreaRect(contour)
        (cx, cy), (width, height), angle = rect
        
        # Compute area ratio
        contour_area = cv2.contourArea(contour)
        rect_area = width * height
        
        if rect_area > 0:
            ratio = contour_area / rect_area
            
            if 1 - tolerance <= ratio <= 1 + tolerance:
                rectangles.append((cx, cy, width, height, angle))
    
    return rectangles


def detect_rounded_rectangles(
    mask: np.ndarray, 
    tolerance: float = 0.1
) -> list:
    """
    Detect rounded rectangles.
    
    Returns:
        List of rounded rects: [(cx, cy, width, height, rx, ry, angle), ...]
    """
    # First detect rectangles
    rects = detect_rectangles(mask, tolerance * 2)  # Looser tolerance
    
    rounded_rects = []
    for cx, cy, width, height, angle in rects:
        # Estimate corner radius by analyzing corners
        rx, ry = estimate_corner_radius(mask, cx, cy, width, height, angle)
        
        if rx > 0 or ry > 0:
            rounded_rects.append((cx, cy, width, height, rx, ry, angle))
    
    return rounded_rects


def estimate_corner_radius(mask, cx, cy, width, height, angle):
    """Estimate corner radius of a rounded rectangle."""
    # Rotate mask to align rectangle
    M = cv2.getRotationMatrix2D((cx, cy), -angle, 1)
    rotated = cv2.warpAffine(mask.astype(np.uint8), M, mask.shape[::-1])
    
    # Extract corners
    x1, y1 = int(cx - width/2), int(cy - height/2)
    x2, y2 = int(cx + width/2), int(cy + height/2)
    
    # Analyze top-left corner
    corner_size = min(width, height) // 4
    corner = rotated[y1:y1+corner_size, x1:x1+corner_size]
    
    # Find the curve in the corner
    contours, _ = cv2.findContours(corner, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_NONE)
    
    if contours:
        # Fit circle to corner curve
        points = contours[0].squeeze()
        if len(points) > 3:
            (ccx, ccy), radius = cv2.minEnclosingCircle(points)
            return radius, radius  # Assuming symmetric corners
    
    return 0, 0
```

### 3.3 SVG Primitive Generation

```python
def shape_to_svg(shape_type: str, params: tuple, color: str) -> str:
    """Convert detected shape to SVG element."""
    
    if shape_type == 'circle':
        cx, cy, r = params
        return f'<circle cx="{cx:.2f}" cy="{cy:.2f}" r="{r:.2f}" fill="{color}"/>'
    
    elif shape_type == 'ellipse':
        cx, cy, rx, ry, angle = params
        transform = f'rotate({angle:.1f} {cx:.2f} {cy:.2f})' if angle != 0 else ''
        return (
            f'<ellipse cx="{cx:.2f}" cy="{cy:.2f}" rx="{rx:.2f}" ry="{ry:.2f}" '
            f'fill="{color}" transform="{transform}"/>'
        )
    
    elif shape_type == 'rect':
        cx, cy, width, height, angle = params
        x, y = cx - width/2, cy - height/2
        transform = f'rotate({angle:.1f} {cx:.2f} {cy:.2f})' if angle != 0 else ''
        return (
            f'<rect x="{x:.2f}" y="{y:.2f}" width="{width:.2f}" height="{height:.2f}" '
            f'fill="{color}" transform="{transform}"/>'
        )
    
    elif shape_type == 'rounded_rect':
        cx, cy, width, height, rx, ry, angle = params
        x, y = cx - width/2, cy - height/2
        transform = f'rotate({angle:.1f} {cx:.2f} {cy:.2f})' if angle != 0 else ''
        return (
            f'<rect x="{x:.2f}" y="{y:.2f}" width="{width:.2f}" height="{height:.2f}" '
            f'rx="{rx:.2f}" ry="{ry:.2f}" fill="{color}" transform="{transform}"/>'
        )
    
    else:
        raise ValueError(f"Unknown shape type: {shape_type}")
```

---

## 4. SAM 2 Integration

### 4.1 Installation

```bash
pip install segment-anything-2
```

### 4.2 Usage

```python
from sam2.build_sam import build_sam2
from sam2.sam2_image_predictor import SAM2ImagePredictor

class SAM2Segmenter:
    def __init__(self, model_size='large'):
        """
        Initialize SAM2 segmenter.
        
        Args:
            model_size: 'tiny', 'small', 'base_plus', 'large'
        """
        model_cfg = {
            'tiny': 'sam2_hiera_t.yaml',
            'small': 'sam2_hiera_s.yaml',
            'base_plus': 'sam2_hiera_b+.yaml',
            'large': 'sam2_hiera_l.yaml',
        }[model_size]
        
        checkpoint = f'sam2_{model_size}.pt'
        
        self.model = build_sam2(model_cfg, checkpoint)
        self.predictor = SAM2ImagePredictor(self.model)
    
    def segment_image(self, image: np.ndarray) -> list:
        """
        Automatically segment image into regions.
        
        Args:
            image: RGB image (H, W, 3)
        
        Returns:
            List of binary masks
        """
        self.predictor.set_image(image)
        
        # Generate automatic masks
        masks = []
        
        # Grid of point prompts
        h, w = image.shape[:2]
        grid_size = 32
        
        for y in range(0, h, grid_size):
            for x in range(0, w, grid_size):
                point = np.array([[x, y]])
                label = np.array([1])  # Foreground
                
                mask, score, _ = self.predictor.predict(
                    point_coords=point,
                    point_labels=label,
                    multimask_output=True
                )
                
                # Take highest scoring mask
                best_idx = np.argmax(score)
                if score[best_idx] > 0.5:
                    masks.append({
                        'mask': mask[best_idx],
                        'score': score[best_idx],
                        'area': np.sum(mask[best_idx])
                    })
        
        # Deduplicate and filter masks
        filtered = self.filter_masks(masks)
        
        return [m['mask'] for m in filtered]
    
    def filter_masks(self, masks: list, iou_threshold: float = 0.8) -> list:
        """Remove duplicate and overlapping masks."""
        if not masks:
            return []
        
        # Sort by score
        masks.sort(key=lambda x: x['score'], reverse=True)
        
        filtered = []
        for mask in masks:
            # Check overlap with existing masks
            is_duplicate = False
            for existing in filtered:
                iou = self.compute_iou(mask['mask'], existing['mask'])
                if iou > iou_threshold:
                    is_duplicate = True
                    break
            
            if not is_duplicate:
                filtered.append(mask)
        
        return filtered
    
    def compute_iou(self, mask1: np.ndarray, mask2: np.ndarray) -> float:
        """Compute Intersection over Union."""
        intersection = np.logical_and(mask1, mask2).sum()
        union = np.logical_or(mask1, mask2).sum()
        return intersection / (union + 1e-8)
```

---

## 5. SVGO Python Wrapper

### 5.1 Complete Implementation

```python
import subprocess
import json
import tempfile
import os
from pathlib import Path

class SVGOptimizer:
    """Python wrapper for SVGO with sensible defaults for vectorization."""
    
    DEFAULT_CONFIG = {
        "multipass": True,
        "floatPrecision": 2,
        "plugins": [
            # Enable default preset
            {
                "name": "preset-default",
                "params": {
                    "overrides": {
                        # Keep viewBox, remove width/height
                        "removeViewBox": False,
                        # Aggressive path optimization
                        "convertPathData": {
                            "floatPrecision": 2,
                            "transformPrecision": 2,
                            "makeArcs": {
                                "threshold": 2.5,
                                "tolerance": 0.5
                            },
                            "straightCurves": True,
                            "lineShorthands": True,
                            "curveSmoothShorthands": True,
                            "removeUseless": True,
                            "collapseRepeated": True,
                            "utilizeAbsolute": True,
                            "negativeExtraSpace": True,
                            "forceAbsolutePath": False
                        },
                        # Merge similar paths
                        "mergePaths": {
                            "force": True
                        },
                        # Collapse groups
                        "collapseGroups": True,
                        # Convert shapes to paths
                        "convertShapeToPath": False,  # Keep shape primitives
                        # Remove empty elements
                        "removeEmptyContainers": True,
                        "removeEmptyAttrs": True,
                        # Optimize colors
                        "convertColors": {
                            "names2hex": True,
                            "rgb2hex": True,
                            "shorthex": True,
                            "shortname": True
                        }
                    }
                }
            },
            # Additional optimizations
            {"name": "removeXMLNS"},
            {"name": "removeDimensions"},
            {"name": "sortAttrs"},
            {"name": "sortDefsChildren"}
        ]
    }
    
    def __init__(self, config: dict = None):
        """
        Initialize optimizer with custom config.
        
        Args:
            config: SVGO configuration dictionary (uses defaults if None)
        """
        self.config = config or self.DEFAULT_CONFIG
        self._check_svgo_installed()
    
    def _check_svgo_installed(self):
        """Verify SVGO is available."""
        try:
            result = subprocess.run(
                ['npx', 'svgo', '--version'],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                raise RuntimeError("SVGO not found. Install with: npm install -g svgo")
        except FileNotFoundError:
            raise RuntimeError("Node.js/npm not found. Please install Node.js.")
    
    def optimize(self, svg_content: str) -> str:
        """
        Optimize SVG content.
        
        Args:
            svg_content: Input SVG as string
        
        Returns:
            Optimized SVG content
        """
        with tempfile.TemporaryDirectory() as tmpdir:
            input_path = Path(tmpdir) / 'input.svg'
            output_path = Path(tmpdir) / 'output.svg'
            config_path = Path(tmpdir) / 'svgo.config.js'
            
            # Write input
            input_path.write_text(svg_content)
            
            # Write config
            config_js = f"module.exports = {json.dumps(self.config)};"
            config_path.write_text(config_js)
            
            # Run SVGO
            result = subprocess.run(
                [
                    'npx', 'svgo',
                    str(input_path),
                    '-o', str(output_path),
                    '--config', str(config_path)
                ],
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                raise RuntimeError(f"SVGO error: {result.stderr}")
            
            return output_path.read_text()
    
    def optimize_file(self, input_path: str, output_path: str = None) -> str:
        """
        Optimize SVG file.
        
        Args:
            input_path: Path to input SVG
            output_path: Path for output (overwrites input if None)
        
        Returns:
            Path to optimized file
        """
        input_path = Path(input_path)
        output_path = Path(output_path) if output_path else input_path
        
        with tempfile.NamedTemporaryFile(
            mode='w', suffix='.config.js', delete=False
        ) as f:
            f.write(f"module.exports = {json.dumps(self.config)};")
            config_path = f.name
        
        try:
            result = subprocess.run(
                [
                    'npx', 'svgo',
                    str(input_path),
                    '-o', str(output_path),
                    '--config', config_path
                ],
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                raise RuntimeError(f"SVGO error: {result.stderr}")
            
            return str(output_path)
        finally:
            os.unlink(config_path)
    
    def get_stats(self, original: str, optimized: str) -> dict:
        """
        Get optimization statistics.
        
        Args:
            original: Original SVG content
            optimized: Optimized SVG content
        
        Returns:
            Statistics dictionary
        """
        original_size = len(original.encode('utf-8'))
        optimized_size = len(optimized.encode('utf-8'))
        
        return {
            'original_size': original_size,
            'optimized_size': optimized_size,
            'reduction_bytes': original_size - optimized_size,
            'reduction_percent': (1 - optimized_size / original_size) * 100
        }


# Pure Python alternative using scour
class ScourOptimizer:
    """Pure Python SVG optimizer using scour (less powerful than SVGO)."""
    
    def __init__(self):
        try:
            import scour
        except ImportError:
            raise ImportError("Install scour: pip install scour")
    
    def optimize(self, svg_content: str) -> str:
        """Optimize SVG content using scour."""
        from scour import scour
        from io import StringIO
        
        options = scour.parse_args([
            '--enable-viewboxing',
            '--enable-id-stripping',
            '--enable-comment-stripping',
            '--shorten-ids',
            '--indent=none',
            '--no-line-breaks',
            '--set-precision=2'
        ])
        
        input_stream = StringIO(svg_content)
        output_stream = StringIO()
        
        scour.start(options, input_stream, output_stream)
        
        return output_stream.getvalue()
```

---

## 6. Complete Integration Example

### 6.1 Enhanced Vectalab Vectorizer

```python
"""
Enhanced Vectalab vectorizer with SOTA techniques.
"""

import numpy as np
from typing import Optional, Dict, Any
from pathlib import Path

class EnhancedVectorizer:
    """
    SOTA vectorizer combining:
    - SAM2 segmentation
    - DiffVG optimization
    - Layered vectorization
    - Shape primitive detection
    - SVGO post-processing
    """
    
    def __init__(
        self,
        sam_model: str = 'large',
        use_diffvg: bool = True,
        detect_primitives: bool = True,
        optimize_output: bool = True
    ):
        self.segmenter = SAM2Segmenter(sam_model)
        self.use_diffvg = use_diffvg
        self.detect_primitives = detect_primitives
        self.optimizer = SVGOptimizer() if optimize_output else None
        
        if use_diffvg:
            self.diffvg_optimizer = DiffVGOptimizer()
    
    def vectorize(
        self,
        image: np.ndarray,
        output_path: Optional[str] = None,
        config: Optional[Dict[str, Any]] = None
    ) -> str:
        """
        Vectorize image to SVG.
        
        Args:
            image: Input RGB image (H, W, 3)
            output_path: Optional path to save SVG
            config: Optional configuration overrides
        
        Returns:
            SVG content as string
        """
        config = config or {}
        
        # Step 1: Segment image
        masks = self.segmenter.segment_image(image)
        
        # Step 2: Process each region
        svg_elements = []
        
        for i, mask in enumerate(masks):
            # Get region color
            color = self._get_region_color(image, mask)
            
            # Try to detect shape primitives
            if self.detect_primitives:
                primitive = self._detect_primitive(mask)
                if primitive:
                    svg_elements.append(
                        shape_to_svg(primitive['type'], primitive['params'], color)
                    )
                    continue
            
            # Fall back to path tracing
            path = self._trace_region(mask)
            
            if self.use_diffvg:
                # Optimize path with DiffVG
                target = image * mask[:, :, np.newaxis]
                path = self.diffvg_optimizer.optimize_path(path, target)
            
            svg_elements.append(self._path_to_svg(path, color))
        
        # Step 3: Compose SVG
        h, w = image.shape[:2]
        svg = self._compose_svg(svg_elements, w, h)
        
        # Step 4: Optimize output
        if self.optimizer:
            svg = self.optimizer.optimize(svg)
        
        # Step 5: Save if path provided
        if output_path:
            Path(output_path).write_text(svg)
        
        return svg
    
    def _get_region_color(self, image: np.ndarray, mask: np.ndarray) -> str:
        """Get average color of masked region as hex."""
        masked_pixels = image[mask]
        avg_color = np.mean(masked_pixels, axis=0).astype(int)
        return f"#{avg_color[0]:02x}{avg_color[1]:02x}{avg_color[2]:02x}"
    
    def _detect_primitive(self, mask: np.ndarray) -> Optional[Dict]:
        """Try to detect a shape primitive in the mask."""
        # Try circles first (strictest)
        circles = detect_circles(mask, tolerance=0.05)
        if circles:
            cx, cy, r = circles[0]
            return {'type': 'circle', 'params': (cx, cy, r)}
        
        # Try ellipses
        ellipses = detect_ellipses(mask, tolerance=0.08)
        if ellipses:
            return {'type': 'ellipse', 'params': ellipses[0]}
        
        # Try rectangles
        rects = detect_rectangles(mask, tolerance=0.08)
        if rects:
            return {'type': 'rect', 'params': rects[0]}
        
        # Try rounded rectangles
        rounded = detect_rounded_rectangles(mask, tolerance=0.1)
        if rounded:
            return {'type': 'rounded_rect', 'params': rounded[0]}
        
        return None
    
    def _trace_region(self, mask: np.ndarray) -> np.ndarray:
        """Trace mask boundary to get path points."""
        import cv2
        
        contours, _ = cv2.findContours(
            mask.astype(np.uint8),
            cv2.RETR_EXTERNAL,
            cv2.CHAIN_APPROX_SIMPLE
        )
        
        if not contours:
            return np.array([])
        
        # Get largest contour
        contour = max(contours, key=cv2.contourArea)
        
        # Simplify
        epsilon = 0.005 * cv2.arcLength(contour, True)
        simplified = cv2.approxPolyDP(contour, epsilon, True)
        
        return simplified.squeeze()
    
    def _path_to_svg(self, points: np.ndarray, color: str) -> str:
        """Convert path points to SVG path element."""
        if len(points) < 3:
            return ""
        
        d = f"M{points[0][0]:.2f},{points[0][1]:.2f}"
        
        for point in points[1:]:
            d += f" L{point[0]:.2f},{point[1]:.2f}"
        
        d += " Z"
        
        return f'<path d="{d}" fill="{color}"/>'
    
    def _compose_svg(
        self, 
        elements: list, 
        width: int, 
        height: int
    ) -> str:
        """Compose SVG document from elements."""
        svg = [
            f'<svg xmlns="http://www.w3.org/2000/svg" '
            f'viewBox="0 0 {width} {height}">'
        ]
        svg.extend(elements)
        svg.append('</svg>')
        
        return '\n'.join(svg)
```

---

## 7. Performance Optimization

### 7.1 GPU Memory Management

```python
import torch
import gc

class MemoryManager:
    """Manage GPU memory for large image processing."""
    
    @staticmethod
    def clear_cache():
        """Clear GPU memory cache."""
        if torch.cuda.is_available():
            torch.cuda.empty_cache()
            gc.collect()
    
    @staticmethod
    def get_optimal_batch_size(image_size: tuple, model_memory: int) -> int:
        """
        Calculate optimal batch size based on available memory.
        
        Args:
            image_size: (height, width) of images
            model_memory: Memory used by model in bytes
        
        Returns:
            Optimal batch size
        """
        if not torch.cuda.is_available():
            return 1
        
        available = torch.cuda.get_device_properties(0).total_memory
        used = torch.cuda.memory_allocated()
        free = available - used - model_memory
        
        # Estimate memory per image (3 channels, float32)
        h, w = image_size
        per_image = h * w * 3 * 4 * 2  # x2 for gradients
        
        return max(1, int(free * 0.8 / per_image))


def process_large_image(
    image: np.ndarray,
    vectorizer: EnhancedVectorizer,
    tile_size: int = 1024,
    overlap: int = 64
) -> str:
    """
    Process large images by tiling.
    
    Args:
        image: Large input image
        vectorizer: Vectorizer instance
        tile_size: Size of each tile
        overlap: Overlap between tiles
    
    Returns:
        Combined SVG
    """
    h, w = image.shape[:2]
    
    if h <= tile_size and w <= tile_size:
        return vectorizer.vectorize(image)
    
    tiles = []
    
    for y in range(0, h, tile_size - overlap):
        for x in range(0, w, tile_size - overlap):
            # Extract tile
            y_end = min(y + tile_size, h)
            x_end = min(x + tile_size, w)
            tile = image[y:y_end, x:x_end]
            
            # Vectorize tile
            svg = vectorizer.vectorize(tile)
            
            tiles.append({
                'svg': svg,
                'x': x,
                'y': y
            })
            
            MemoryManager.clear_cache()
    
    # Combine tiles
    return combine_svg_tiles(tiles, w, h)


def combine_svg_tiles(tiles: list, width: int, height: int) -> str:
    """Combine SVG tiles into single document."""
    svg_parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" '
        f'viewBox="0 0 {width} {height}">'
    ]
    
    for tile in tiles:
        # Extract elements from tile SVG
        elements = extract_svg_elements(tile['svg'])
        
        # Translate to correct position
        x, y = tile['x'], tile['y']
        svg_parts.append(f'<g transform="translate({x},{y})">')
        svg_parts.extend(elements)
        svg_parts.append('</g>')
    
    svg_parts.append('</svg>')
    
    return '\n'.join(svg_parts)
```

---

## 8. Testing and Validation

### 8.1 Quality Metrics

```python
import numpy as np
from skimage.metrics import structural_similarity as ssim
from PIL import Image
import cairosvg

def compute_metrics(original: np.ndarray, svg: str) -> dict:
    """
    Compute quality metrics between original image and rendered SVG.
    
    Args:
        original: Original RGB image
        svg: SVG content string
    
    Returns:
        Dictionary of metrics
    """
    # Render SVG to image
    h, w = original.shape[:2]
    png_data = cairosvg.svg2png(
        bytestring=svg.encode(),
        output_width=w,
        output_height=h
    )
    rendered = np.array(Image.open(io.BytesIO(png_data)).convert('RGB'))
    
    # SSIM
    ssim_value = ssim(original, rendered, channel_axis=2)
    
    # PSNR
    mse = np.mean((original.astype(float) - rendered.astype(float)) ** 2)
    psnr = 10 * np.log10(255 ** 2 / (mse + 1e-10))
    
    # Path count
    path_count = svg.count('<path')
    
    # File size
    file_size = len(svg.encode('utf-8'))
    
    return {
        'ssim': ssim_value,
        'psnr': psnr,
        'path_count': path_count,
        'file_size_bytes': file_size
    }


def run_benchmark(
    vectorizer: EnhancedVectorizer,
    test_images: list
) -> dict:
    """
    Run benchmark on test images.
    
    Args:
        vectorizer: Vectorizer to benchmark
        test_images: List of (name, image) tuples
    
    Returns:
        Benchmark results
    """
    results = []
    
    for name, image in test_images:
        import time
        
        start = time.time()
        svg = vectorizer.vectorize(image)
        elapsed = time.time() - start
        
        metrics = compute_metrics(image, svg)
        metrics['name'] = name
        metrics['time_seconds'] = elapsed
        
        results.append(metrics)
    
    # Aggregate
    avg_ssim = np.mean([r['ssim'] for r in results])
    avg_psnr = np.mean([r['psnr'] for r in results])
    avg_time = np.mean([r['time_seconds'] for r in results])
    
    return {
        'individual': results,
        'average': {
            'ssim': avg_ssim,
            'psnr': avg_psnr,
            'time_seconds': avg_time
        }
    }
```

---

*This document provides implementation-specific details for the SOTA Vectorization Research Report.*
*Last updated: June 2025*
