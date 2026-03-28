use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, Pixel, Rgba, RgbaImage};
use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Serialize};

pub const BENCHMARK_REFERENCE_MIN_LONGEST_SIDE: u32 = 1024;
const MAX_PSNR_DB: f64 = 100.0;

#[derive(Debug, Clone)]
pub(crate) struct PreparedMetricsInput {
    flattened: RgbaImage,
    gray: Vec<f64>,
    blurred_gray: Vec<f64>,
    dilated_edges: Vec<bool>,
    foreground_mask: Vec<bool>,
    topology_components: usize,
    topology_holes: usize,
    lab_pixels: Vec<(f64, f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub ssim: f64,
    pub ssim_perceptual: f64,
    pub local_ssim_p10: f64,
    pub local_ssim_min: f64,
    pub gradient_similarity: f64,
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
    let prepared = PreparedMetricsInput::from_image(original);
    compute_metrics_against(&prepared, svg)
}

pub(crate) fn compute_metrics_against(
    prepared: &PreparedMetricsInput,
    svg: &str,
) -> Result<QualityMetrics> {
    let rendered =
        render_svg_to_image(svg, prepared.flattened.width(), prepared.flattened.height())?;
    let rendered = flatten_on_white(&rendered);
    let rendered_blurred = image::imageops::blur(&rendered, 1.5);

    let mut mse = 0.0;
    let mut mae = 0.0;
    let mut rendered_gray = Vec::with_capacity((rendered.width() * rendered.height()) as usize);
    let mut rendered_blurred_gray =
        Vec::with_capacity((rendered.width() * rendered.height()) as usize);

    for ((left, right), right_blur) in prepared
        .flattened
        .pixels()
        .zip(rendered.pixels())
        .zip(rendered_blurred.pixels())
    {
        let [lr, lg, lb, _] = left.0;
        let [rr, rg, rb, _] = right.0;
        for (a, b) in [(lr, rr), (lg, rg), (lb, rb)] {
            let delta = f64::from(a) - f64::from(b);
            mse += delta * delta;
            mae += delta.abs();
        }

        rendered_gray.push(luma(right));
        rendered_blurred_gray.push(luma(right_blur));
    }

    let denom =
        (prepared.flattened.width() as f64 * prepared.flattened.height() as f64 * 3.0).max(1.0);
    mse /= denom;
    mae /= denom;
    let psnr = if mse == 0.0 {
        MAX_PSNR_DB
    } else {
        10.0 * ((255.0 * 255.0) / mse).log10()
    };

    let ssim = compute_ssim(&prepared.gray, &rendered_gray);
    let ssim_perceptual = compute_ssim(&prepared.blurred_gray, &rendered_blurred_gray);
    let (local_ssim_p10, local_ssim_min) = compute_local_ssim_distribution(
        &prepared.gray,
        &rendered_gray,
        prepared.flattened.width(),
        prepared.flattened.height(),
    );
    let gradient_similarity = compute_gradient_similarity(
        &prepared.blurred_gray,
        &rendered_blurred_gray,
        prepared.flattened.width(),
        prepared.flattened.height(),
    );
    let edge = edge_metrics_against(prepared, &rendered);
    let delta_e = average_delta_e_against(prepared, &rendered);
    let color_similarity = (1.0 - (delta_e / 40.0).min(1.0)).clamp(0.0, 1.0);
    let foreground_iou = foreground_iou_against(prepared, &rendered);
    let topology_score = topology_score_against(prepared, &rendered);
    let fidelity_score = (ssim * 0.24
        + ssim_perceptual * 0.14
        + gradient_similarity * 0.13
        + edge.f1 * 0.15
        + edge.iou * 0.10
        + foreground_iou * 0.10
        + color_similarity * 0.10
        + topology_score * 0.04)
        .clamp(0.0, 1.0);

    Ok(QualityMetrics {
        ssim,
        ssim_perceptual,
        local_ssim_p10,
        local_ssim_min,
        gradient_similarity,
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
        path_count: count_shape_elements(svg),
    })
}

impl PreparedMetricsInput {
    pub(crate) fn from_image(original: &DynamicImage) -> Self {
        let flattened = flatten_on_white(&original.to_rgba8());
        let flattened_blurred = image::imageops::blur(&flattened, 1.5);
        let gray = grayscale_pixels(&flattened);
        let blurred_gray = grayscale_pixels(&flattened_blurred);
        let detected_edges = detect_edges_from_gray(&gray, flattened.width(), flattened.height());
        let dilated_edges = dilate_binary(&detected_edges, flattened.width(), flattened.height());
        let foreground_mask = threshold_foreground(&flattened);
        let topology_components = count_components(
            &foreground_mask,
            flattened.width(),
            flattened.height(),
            true,
        );
        let topology_holes = count_holes(&foreground_mask, flattened.width(), flattened.height());
        let lab_pixels = flattened
            .pixels()
            .map(|pixel| rgb_to_lab(pixel[0], pixel[1], pixel[2]))
            .collect();
        Self {
            flattened,
            gray,
            blurred_gray,
            dilated_edges,
            foreground_mask,
            topology_components,
            topology_holes,
            lab_pixels,
        }
    }
}

fn count_shape_elements(svg: &str) -> usize {
    [
        "<path",
        "<rect",
        "<circle",
        "<ellipse",
        "<polygon",
        "<polyline",
        "<line",
    ]
    .into_iter()
    .map(|needle| svg.matches(needle).count())
    .sum()
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
    render_svg_file_to_png_with_min_longest_side(input, output, 0)
}

pub fn render_svg_file_to_png_with_min_longest_side(
    input: &std::path::Path,
    output: &std::path::Path,
    min_longest_side: u32,
) -> Result<()> {
    let mut options = usvg::Options {
        resources_dir: input
            .canonicalize()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf())),
        ..usvg::Options::default()
    };
    let data = std::fs::read(input)?;
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_data(&data, &options).map_err(|e| anyhow!("invalid svg: {e}"))?;
    let base_size = tree.size().to_int_size();
    let base_width = base_size.width().max(1);
    let base_height = base_size.height().max(1);
    let longest_side = base_width.max(base_height);
    let scale = if min_longest_side > 0 && longest_side < min_longest_side {
        min_longest_side as f32 / longest_side as f32
    } else {
        1.0
    };
    let width = ((base_width as f32) * scale).round().max(1.0) as u32;
    let height = ((base_height as f32) * scale).round().max(1.0) as u32;
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| anyhow!("cannot allocate pixmap"))?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / base_width as f32,
        height as f32 / base_height as f32,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
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

fn grayscale_pixels(image: &RgbaImage) -> Vec<f64> {
    image.pixels().map(luma).collect()
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

fn compute_gradient_similarity(left: &[f64], right: &[f64], width: u32, height: u32) -> f64 {
    let left_gradients = sobel_magnitudes(left, width, height);
    let right_gradients = sobel_magnitudes(right, width, height);
    compute_ssim(&left_gradients, &right_gradients)
}

fn compute_local_ssim_distribution(
    left: &[f64],
    right: &[f64],
    width: u32,
    height: u32,
) -> (f64, f64) {
    let grid_x = (width / 128).clamp(1, 8);
    let grid_y = (height / 128).clamp(1, 8);
    let tile_width = ((width as f64) / grid_x as f64).ceil() as usize;
    let tile_height = ((height as f64) / grid_y as f64).ceil() as usize;
    let width = width as usize;
    let height = height as usize;
    let mut scores = Vec::new();

    for tile_y in 0..grid_y as usize {
        let y0 = tile_y * tile_height;
        if y0 >= height {
            continue;
        }
        let y1 = ((tile_y + 1) * tile_height).min(height);
        for tile_x in 0..grid_x as usize {
            let x0 = tile_x * tile_width;
            if x0 >= width {
                continue;
            }
            let x1 = ((tile_x + 1) * tile_width).min(width);
            let mut left_tile = Vec::with_capacity((x1 - x0) * (y1 - y0));
            let mut right_tile = Vec::with_capacity((x1 - x0) * (y1 - y0));
            for y in y0..y1 {
                let row_start = y * width;
                left_tile.extend_from_slice(&left[row_start + x0..row_start + x1]);
                right_tile.extend_from_slice(&right[row_start + x0..row_start + x1]);
            }
            scores.push(compute_ssim(&left_tile, &right_tile));
        }
    }

    scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let min = scores.first().copied().unwrap_or(0.0);
    let p10 = percentile(&scores, 0.10);
    (p10, min)
}

fn sobel_magnitudes(gray: &[f64], width: u32, height: u32) -> Vec<f64> {
    let width = width as usize;
    let height = height as usize;
    let mut magnitudes = vec![0.0; gray.len()];
    if width < 3 || height < 3 {
        return magnitudes;
    }

    let index = |x: usize, y: usize| -> usize { y * width + x };
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let sample = |dx: isize, dy: isize| -> f64 {
                gray[index((x as isize + dx) as usize, (y as isize + dy) as usize)]
            };
            let gx = -sample(-1, -1) + sample(1, -1) - 2.0 * sample(-1, 0) + 2.0 * sample(1, 0)
                - sample(-1, 1)
                + sample(1, 1);
            let gy = -sample(-1, -1) - 2.0 * sample(0, -1) - sample(1, -1)
                + sample(-1, 1)
                + 2.0 * sample(0, 1)
                + sample(1, 1);
            magnitudes[index(x, y)] = (gx * gx + gy * gy).sqrt().min(255.0);
        }
    }

    magnitudes
}

struct EdgeMetrics {
    iou: f64,
    precision: f64,
    recall: f64,
    f1: f64,
}

fn edge_metrics_against(prepared: &PreparedMetricsInput, right: &RgbaImage) -> EdgeMetrics {
    let right_edges = detect_edges(right);
    let right_edges = dilate_binary(&right_edges, right.width(), right.height());
    overlap_edge_metrics(&prepared.dilated_edges, &right_edges)
}

fn overlap_edge_metrics(left_edges: &[bool], right_edges: &[bool]) -> EdgeMetrics {
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
    detect_edges_from_gray(&gray, width, height)
}

fn detect_edges_from_gray(gray: &[f64], width: u32, height: u32) -> Vec<bool> {
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

fn topology_score_against(prepared: &PreparedMetricsInput, right: &RgbaImage) -> f64 {
    let (components_right, holes_right) = topology_stats(right);
    let score_c = ratio_score(prepared.topology_components, components_right);
    let score_h = ratio_score(prepared.topology_holes, holes_right);
    (score_c + score_h) / 2.0
}

fn topology_stats(image: &RgbaImage) -> (usize, usize) {
    let binary = threshold_foreground(image);
    let components = count_components(&binary, image.width(), image.height(), true);
    let holes = count_holes(&binary, image.width(), image.height());
    (components, holes)
}

fn threshold_foreground(image: &RgbaImage) -> Vec<bool> {
    image.pixels().map(is_foreground).collect()
}

fn is_foreground(pixel: &Rgba<u8>) -> bool {
    let [r, g, b, _] = pixel.0;
    let luminance = 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b);
    luminance < 250.0
}

fn foreground_iou_against(prepared: &PreparedMetricsInput, right: &RgbaImage) -> f64 {
    let right_mask = threshold_foreground(right);
    foreground_iou_from_masks(&prepared.foreground_mask, &right_mask)
}

fn foreground_iou_from_masks(left_mask: &[bool], right_mask: &[bool]) -> f64 {
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
                visit_neighbor(
                    cx.wrapping_sub(1),
                    cy,
                    width,
                    height,
                    binary,
                    target,
                    &mut visited,
                    &mut queue,
                    idx,
                );
                visit_neighbor(
                    cx + 1,
                    cy,
                    width,
                    height,
                    binary,
                    target,
                    &mut visited,
                    &mut queue,
                    idx,
                );
                visit_neighbor(
                    cx,
                    cy.wrapping_sub(1),
                    width,
                    height,
                    binary,
                    target,
                    &mut visited,
                    &mut queue,
                    idx,
                );
                visit_neighbor(
                    cx,
                    cy + 1,
                    width,
                    height,
                    binary,
                    target,
                    &mut visited,
                    &mut queue,
                    idx,
                );
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
                visit_hole_neighbor(
                    cx.wrapping_sub(1),
                    cy,
                    width,
                    height,
                    binary,
                    &mut visited,
                    &mut queue,
                    idx,
                    &mut touches_border,
                );
                visit_hole_neighbor(
                    cx + 1,
                    cy,
                    width,
                    height,
                    binary,
                    &mut visited,
                    &mut queue,
                    idx,
                    &mut touches_border,
                );
                visit_hole_neighbor(
                    cx,
                    cy.wrapping_sub(1),
                    width,
                    height,
                    binary,
                    &mut visited,
                    &mut queue,
                    idx,
                    &mut touches_border,
                );
                visit_hole_neighbor(
                    cx,
                    cy + 1,
                    width,
                    height,
                    binary,
                    &mut visited,
                    &mut queue,
                    idx,
                    &mut touches_border,
                );
            }
            if !touches_border {
                holes += 1;
            }
        }
    }
    holes
}

fn visit_neighbor(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    binary: &[bool],
    target: bool,
    visited: &mut [bool],
    queue: &mut std::collections::VecDeque<(u32, u32)>,
    idx: impl Fn(u32, u32) -> usize,
) {
    if x < width && y < height {
        let index = idx(x, y);
        if !visited[index] && binary[index] == target {
            visited[index] = true;
            queue.push_back((x, y));
        }
    }
}

fn visit_hole_neighbor(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    binary: &[bool],
    visited: &mut [bool],
    queue: &mut std::collections::VecDeque<(u32, u32)>,
    idx: impl Fn(u32, u32) -> usize,
    touches_border: &mut bool,
) {
    if x < width && y < height {
        let index = idx(x, y);
        if !visited[index] && !binary[index] {
            if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                *touches_border = true;
            }
            visited[index] = true;
            queue.push_back((x, y));
        }
    }
}

fn ratio_score(a: usize, b: usize) -> f64 {
    let max_value = a.max(b);
    if max_value == 0 {
        1.0
    } else {
        1.0 - (a.abs_diff(b) as f64 / max_value as f64)
    }
}

fn percentile(values: &[f64], quantile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return values[0];
    }
    let position = quantile.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let weight = position - lower as f64;
        values[lower] * (1.0 - weight) + values[upper] * weight
    }
}

fn average_delta_e_against(prepared: &PreparedMetricsInput, right: &RgbaImage) -> f64 {
    let rendered_lab = right
        .pixels()
        .map(|pixel| rgb_to_lab(pixel[0], pixel[1], pixel[2]))
        .collect::<Vec<_>>();
    average_delta_e_from_lab_against(prepared, &rendered_lab)
}

fn average_delta_e_from_lab_against(
    prepared: &PreparedMetricsInput,
    rendered_lab: &[(f64, f64, f64)],
) -> f64 {
    let mut total = 0.0;
    let mut count = 0usize;
    for (lab_a, lab_b) in prepared.lab_pixels.iter().zip(rendered_lab.iter()) {
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

#[cfg(test)]
mod tests {
    use std::fs;

    use image::{DynamicImage, Rgba, RgbaImage};
    use tempfile::tempdir;

    use super::{
        compute_local_ssim_distribution, compute_metrics, count_shape_elements,
        render_svg_file_to_png_with_min_longest_side, BENCHMARK_REFERENCE_MIN_LONGEST_SIDE,
    };

    #[test]
    fn benchmark_reference_render_upscales_small_svg_without_distorting_aspect_ratio() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("icon.svg");
        let output = dir.path().join("icon.png");
        fs::write(
            &input,
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12" viewBox="0 0 24 12"><rect width="24" height="12" fill="#000"/></svg>"##,
        )
        .unwrap();

        render_svg_file_to_png_with_min_longest_side(
            &input,
            &output,
            BENCHMARK_REFERENCE_MIN_LONGEST_SIDE,
        )
        .unwrap();

        let image = image::open(output).unwrap();
        assert_eq!(image.width(), BENCHMARK_REFERENCE_MIN_LONGEST_SIDE);
        assert_eq!(image.height(), BENCHMARK_REFERENCE_MIN_LONGEST_SIDE / 2);
    }

    #[test]
    fn local_ssim_detects_single_quadrant_failure() {
        let width = 256;
        let height = 256;
        let mut left = vec![255.0; width * height];
        let mut right = left.clone();
        for y in 0..height / 2 {
            for x in 0..width / 2 {
                right[y * width + x] = 0.0;
            }
        }

        let (p10, min) =
            compute_local_ssim_distribution(&left, &right, width as u32, height as u32);
        assert!(p10 < 0.6);
        assert!(min < 0.1);
        left[0] = 255.0;
    }

    #[test]
    fn psnr_is_finite_for_perfect_reconstruction() {
        let mut image = RgbaImage::new(4, 4);
        for pixel in image.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 255]);
        }

        let metrics = compute_metrics(
            &DynamicImage::ImageRgba8(image),
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4"><rect width="4" height="4" fill="#000"/></svg>"##,
        )
        .unwrap();

        assert!(metrics.psnr.is_finite());
        assert_eq!(metrics.psnr, 100.0);
    }

    #[test]
    fn path_count_includes_native_shape_elements() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg"><rect width="4" height="4"/><circle cx="2" cy="2" r="1"/><path d="M0 0L1 1Z"/></svg>"##;
        assert_eq!(count_shape_elements(svg), 3);
    }
}
