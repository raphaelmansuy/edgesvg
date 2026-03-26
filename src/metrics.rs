use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, Pixel, Rgba, RgbaImage};
use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub ssim: f64,
    pub ssim_perceptual: f64,
    pub edge_similarity: f64,
    pub edge_precision: f64,
    pub edge_recall: f64,
    pub edge_f1: f64,
    pub foreground_iou: f64,
    pub color_similarity: f64,
    pub fidelity_score: f64,
    pub delta_e: f64,
    pub topology_score: f64,
    pub psnr: f64,
    pub mae: f64,
    pub file_size: usize,
    pub path_count: usize,
}

pub fn compute_metrics(original: &DynamicImage, svg: &str) -> Result<QualityMetrics> {
    let original = flatten_on_white(&original.to_rgba8());
    let rendered = render_svg_to_image(svg, original.width(), original.height())?;
    let rendered = flatten_on_white(&rendered);
    let original_blurred = image::imageops::blur(&original, 1.5);
    let rendered_blurred = image::imageops::blur(&rendered, 1.5);

    let mut mse = 0.0;
    let mut mae = 0.0;
    let mut original_gray = Vec::with_capacity((original.width() * original.height()) as usize);
    let mut rendered_gray = Vec::with_capacity((rendered.width() * rendered.height()) as usize);
    let mut original_blurred_gray =
        Vec::with_capacity((original.width() * original.height()) as usize);
    let mut rendered_blurred_gray =
        Vec::with_capacity((rendered.width() * rendered.height()) as usize);

    for (((left, right), left_blur), right_blur) in original
        .pixels()
        .zip(rendered.pixels())
        .zip(original_blurred.pixels())
        .zip(rendered_blurred.pixels())
    {
        let [lr, lg, lb, _] = left.0;
        let [rr, rg, rb, _] = right.0;
        for (a, b) in [(lr, rr), (lg, rg), (lb, rb)] {
            let delta = f64::from(a) - f64::from(b);
            mse += delta * delta;
            mae += delta.abs();
        }

        original_gray.push(luma(left));
        rendered_gray.push(luma(right));
        original_blurred_gray.push(luma(left_blur));
        rendered_blurred_gray.push(luma(right_blur));
    }

    let denom = (original.width() as f64 * original.height() as f64 * 3.0).max(1.0);
    mse /= denom;
    mae /= denom;
    let psnr = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * ((255.0 * 255.0) / mse).log10()
    };

    let ssim = compute_ssim(&original_gray, &rendered_gray);
    let ssim_perceptual = compute_ssim(&original_blurred_gray, &rendered_blurred_gray);
    let edge = edge_metrics(&original, &rendered);
    let delta_e = average_delta_e(&original, &rendered);
    let color_similarity = (1.0 - (delta_e / 40.0).min(1.0)).clamp(0.0, 1.0);
    let foreground_iou = foreground_iou(&original, &rendered);
    let topology_score = topology_score(&original, &rendered);
    let fidelity_score = (ssim * 0.28
        + ssim_perceptual * 0.17
        + edge.f1 * 0.18
        + edge.iou * 0.12
        + foreground_iou * 0.10
        + color_similarity * 0.10
        + topology_score * 0.05)
        .clamp(0.0, 1.0);

    Ok(QualityMetrics {
        ssim,
        ssim_perceptual,
        edge_similarity: edge.iou,
        edge_precision: edge.precision,
        edge_recall: edge.recall,
        edge_f1: edge.f1,
        foreground_iou,
        color_similarity,
        fidelity_score,
        delta_e,
        topology_score,
        psnr,
        mae,
        file_size: svg.len(),
        path_count: svg.matches("<path").count(),
    })
}

pub fn render_svg_to_image(svg: &str, width: u32, height: u32) -> Result<RgbaImage> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| anyhow!("invalid svg: {e}"))?;

    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| anyhow!("cannot allocate pixmap"))?;
    let base_width = tree.size().width().max(1.0);
    let base_height = tree.size().height().max(1.0);
    let transform =
        tiny_skia::Transform::from_scale(width as f32 / base_width, height as f32 / base_height);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    RgbaImage::from_raw(width, height, pixmap.data().to_vec())
        .context("unable to build rendered image")
}

pub fn render_svg_file_to_png(input: &std::path::Path, output: &std::path::Path) -> Result<()> {
    let mut options = usvg::Options {
        resources_dir: input
            .canonicalize()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf())),
        ..usvg::Options::default()
    };
    options.fontdb_mut().load_system_fonts();
    let data = std::fs::read(input)?;
    let tree = usvg::Tree::from_data(&data, &options).map_err(|e| anyhow!("invalid svg: {e}"))?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow!("cannot allocate pixmap"))?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png(output)?;
    Ok(())
}

fn flatten_on_white(image: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(image.width(), image.height());
    for (target, source) in out.pixels_mut().zip(image.pixels()) {
        let alpha = f64::from(source[3]) / 255.0;
        let blend = |channel: u8| -> u8 {
            let channel = f64::from(channel);
            ((channel * alpha) + 255.0 * (1.0 - alpha))
                .round()
                .clamp(0.0, 255.0) as u8
        };
        *target = Rgba([blend(source[0]), blend(source[1]), blend(source[2]), 255]);
    }
    out
}

fn luma(pixel: &Rgba<u8>) -> f64 {
    let channels = pixel.channels();
    0.299 * f64::from(channels[0]) + 0.587 * f64::from(channels[1]) + 0.114 * f64::from(channels[2])
}

fn compute_ssim(left: &[f64], right: &[f64]) -> f64 {
    let n = left.len().min(right.len());
    if n == 0 {
        return 0.0;
    }

    let mean_left = left.iter().take(n).sum::<f64>() / n as f64;
    let mean_right = right.iter().take(n).sum::<f64>() / n as f64;

    let mut var_left = 0.0;
    let mut var_right = 0.0;
    let mut covariance = 0.0;

    for (&a, &b) in left.iter().zip(right.iter()).take(n) {
        var_left += (a - mean_left).powi(2);
        var_right += (b - mean_right).powi(2);
        covariance += (a - mean_left) * (b - mean_right);
    }

    let denom = (n as f64 - 1.0).max(1.0);
    var_left /= denom;
    var_right /= denom;
    covariance /= denom;

    let c1 = (0.01_f64 * 255.0).powi(2);
    let c2 = (0.03_f64 * 255.0).powi(2);
    let value = ((2.0 * mean_left * mean_right + c1) * (2.0 * covariance + c2))
        / ((mean_left.powi(2) + mean_right.powi(2) + c1) * (var_left + var_right + c2));
    value.clamp(0.0, 1.0)
}

struct EdgeMetrics {
    iou: f64,
    precision: f64,
    recall: f64,
    f1: f64,
}

fn edge_metrics(left: &RgbaImage, right: &RgbaImage) -> EdgeMetrics {
    let left_edges = detect_edges(left);
    let right_edges = detect_edges(right);
    let left_edges = dilate_binary(&left_edges, left.width(), left.height());
    let right_edges = dilate_binary(&right_edges, right.width(), right.height());

    let mut intersection = 0usize;
    let mut union = 0usize;
    let mut predicted = 0usize;
    let mut reference = 0usize;
    for (a, b) in left_edges.iter().zip(right_edges.iter()) {
        if *a || *b {
            union += 1;
        }
        if *a && *b {
            intersection += 1;
        }
        if *b {
            predicted += 1;
        }
        if *a {
            reference += 1;
        }
    }
    let iou = if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    };
    let precision = if predicted == 0 {
        1.0
    } else {
        intersection as f64 / predicted as f64
    };
    let recall = if reference == 0 {
        1.0
    } else {
        intersection as f64 / reference as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    EdgeMetrics {
        iou,
        precision,
        recall,
        f1,
    }
}

fn detect_edges(image: &RgbaImage) -> Vec<bool> {
    let (width, height) = image.dimensions();
    let gray = image.pixels().map(luma).collect::<Vec<_>>();
    let mut edges = vec![false; (width * height) as usize];
    if width < 3 || height < 3 {
        return edges;
    }

    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let p = |dx: i32, dy: i32| -> f64 {
                gray[idx((x as i32 + dx) as u32, (y as i32 + dy) as u32)]
            };
            let gx = -p(-1, -1) + p(1, -1) - 2.0 * p(-1, 0) + 2.0 * p(1, 0) - p(-1, 1) + p(1, 1);
            let gy = -p(-1, -1) - 2.0 * p(0, -1) - p(1, -1) + p(-1, 1) + 2.0 * p(0, 1) + p(1, 1);
            let magnitude = (gx * gx + gy * gy).sqrt();
            if magnitude > 100.0 {
                edges[idx(x, y)] = true;
            }
        }
    }
    edges
}

fn dilate_binary(input: &[bool], width: u32, height: u32) -> Vec<bool> {
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };
    let mut output = vec![false; input.len()];
    for y in 0..height {
        for x in 0..width {
            let mut on = false;
            for dy in -1_i32..=1 {
                for dx in -1_i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < width as i32
                        && ny < height as i32
                        && input[idx(nx as u32, ny as u32)]
                    {
                        on = true;
                    }
                }
            }
            output[idx(x, y)] = on;
        }
    }
    output
}

fn topology_score(left: &RgbaImage, right: &RgbaImage) -> f64 {
    let (components_left, holes_left) = topology_stats(left);
    let (components_right, holes_right) = topology_stats(right);

    let score_c = ratio_score(components_left, components_right);
    let score_h = ratio_score(holes_left, holes_right);
    (score_c + score_h) / 2.0
}

fn topology_stats(image: &RgbaImage) -> (usize, usize) {
    let binary = threshold_foreground(image);
    let components = count_components(&binary, image.width(), image.height(), true);
    let holes = count_holes(&binary, image.width(), image.height());
    (components, holes)
}

fn threshold_foreground(image: &RgbaImage) -> Vec<bool> {
    image
        .pixels()
        .map(|pixel| {
            let [r, g, b, _] = pixel.0;
            let luminance = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b);
            luminance < 250.0
        })
        .collect()
}

fn foreground_iou(left: &RgbaImage, right: &RgbaImage) -> f64 {
    let left_mask = threshold_foreground(left);
    let right_mask = threshold_foreground(right);
    let mut intersection = 0usize;
    let mut union = 0usize;
    for (a, b) in left_mask.iter().zip(right_mask.iter()) {
        if *a || *b {
            union += 1;
        }
        if *a && *b {
            intersection += 1;
        }
    }
    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

fn count_components(binary: &[bool], width: u32, height: u32, target: bool) -> usize {
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };
    let mut visited = vec![false; binary.len()];
    let mut count = 0usize;
    let mut queue = std::collections::VecDeque::new();

    for y in 0..height {
        for x in 0..width {
            let index = idx(x, y);
            if visited[index] || binary[index] != target {
                continue;
            }
            count += 1;
            visited[index] = true;
            queue.push_back((x, y));
            while let Some((cx, cy)) = queue.pop_front() {
                for (nx, ny) in neighbors4(cx, cy, width, height) {
                    let nindex = idx(nx, ny);
                    if !visited[nindex] && binary[nindex] == target {
                        visited[nindex] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }
    }

    count
}

fn count_holes(binary: &[bool], width: u32, height: u32) -> usize {
    let idx = |x: u32, y: u32| -> usize { (y * width + x) as usize };
    let mut visited = vec![false; binary.len()];
    let mut holes = 0usize;
    let mut queue = std::collections::VecDeque::new();

    for y in 0..height {
        for x in 0..width {
            let index = idx(x, y);
            if visited[index] || binary[index] {
                continue;
            }
            let mut touches_border = x == 0 || y == 0 || x + 1 == width || y + 1 == height;
            visited[index] = true;
            queue.push_back((x, y));
            while let Some((cx, cy)) = queue.pop_front() {
                for (nx, ny) in neighbors4(cx, cy, width, height) {
                    let nindex = idx(nx, ny);
                    if !visited[nindex] && !binary[nindex] {
                        if nx == 0 || ny == 0 || nx + 1 == width || ny + 1 == height {
                            touches_border = true;
                        }
                        visited[nindex] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            if !touches_border {
                holes += 1;
            }
        }
    }
    holes
}

fn neighbors4(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut out = Vec::with_capacity(4);
    if x > 0 {
        out.push((x - 1, y));
    }
    if x + 1 < width {
        out.push((x + 1, y));
    }
    if y > 0 {
        out.push((x, y - 1));
    }
    if y + 1 < height {
        out.push((x, y + 1));
    }
    out
}

fn ratio_score(a: usize, b: usize) -> f64 {
    let max_value = a.max(b);
    if max_value == 0 {
        1.0
    } else {
        1.0 - (a.abs_diff(b) as f64 / max_value as f64)
    }
}

fn average_delta_e(left: &RgbaImage, right: &RgbaImage) -> f64 {
    let mut total = 0.0;
    let mut count = 0usize;
    for (a, b) in left.pixels().zip(right.pixels()) {
        let lab_a = rgb_to_lab(a[0], a[1], a[2]);
        let lab_b = rgb_to_lab(b[0], b[1], b[2]);
        total += ((lab_a.0 - lab_b.0).powi(2)
            + (lab_a.1 - lab_b.1).powi(2)
            + (lab_a.2 - lab_b.2).powi(2))
        .sqrt();
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn rgb_to_lab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let srgb_to_linear = |v: u8| -> f64 {
        let value = f64::from(v) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);

    let x = r * 0.4124 + g * 0.3576 + b * 0.1805;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = r * 0.0193 + g * 0.1192 + b * 0.9505;

    let xr = x / 0.95047;
    let yr = y / 1.0;
    let zr = z / 1.08883;

    let f = |t: f64| -> f64 {
        if t > 0.008856 {
            t.powf(1.0 / 3.0)
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let fx = f(xr);
    let fy = f(yr);
    let fz = f(zr);
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}
