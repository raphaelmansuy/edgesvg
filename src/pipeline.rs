use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::analysis::analyze_image;
use crate::metrics::{compute_metrics_against, PreparedMetricsInput};
use crate::preprocess::{
    adaptive_trace_settings, build_monochrome_alpha_mask, detect_sparse_monochrome_color,
    preprocess_image, quantize_image, MONOCHROME_MASK_ALPHA_THRESHOLD,
};
use crate::svg::{
    optimize_svg, wrap_svg_with_gradient_background, wrap_svg_with_gradient_surface,
    LinearGradientSpec, LinearGradientStop,
};
use crate::types::{Complexity, ImageAnalysis, ImageKind, QualityPreset, VectorizationReport};
use crate::vectorizer::trace_to_svg;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorizeOptions {
    pub target_ssim: f64,
    pub max_file_size: usize,
    pub max_iterations: usize,
    pub quality: Option<QualityPreset>,
}

impl Default for VectorizeOptions {
    fn default() -> Self {
        Self {
            target_ssim: 0.998,
            max_file_size: 100_000,
            max_iterations: 4,
            quality: Some(QualityPreset::Balanced),
        }
    }
}

pub fn vectorize(
    input_path: &Path,
    options: &VectorizeOptions,
) -> Result<(String, VectorizationReport)> {
    let original = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_image(&original, options)
}

pub fn vectorize_image(
    original: &image::DynamicImage,
    options: &VectorizeOptions,
) -> Result<(String, VectorizationReport)> {
    let source_image = image::DynamicImage::ImageRgba8(original.to_rgba8());
    let source_rgba = source_image.to_rgba8();
    let flattened = flatten_transparency_to_white(&source_image.to_rgba8());
    let flattened_image = image::DynamicImage::ImageRgba8(flattened);
    let analysis_image = if has_substantial_transparency(&source_image) {
        &source_image
    } else {
        &flattened_image
    };
    let analysis = analyze_image(analysis_image);
    let trace_input = trace_candidate_image(&source_image, &flattened_image, &analysis)?;
    let mut trace_candidates = vec![trace_input];
    if let Some(mask_candidate) = sparse_monochrome_mask_candidate(&source_rgba, &analysis) {
        trace_candidates.push(mask_candidate);
    }
    if let Some(diagram_candidate) = opaque_diagram_candidate(&source_rgba, &analysis) {
        trace_candidates.push(diagram_candidate);
    }
    if let Some(diagram_candidate) = transparent_diagram_candidate(&source_rgba, &analysis) {
        trace_candidates.push(diagram_candidate);
    }
    if let Some(flat_graphic_candidate) = transparent_flat_graphic_candidate(&source_rgba, &analysis)
    {
        trace_candidates.push(flat_graphic_candidate);
    }
    if let Some(photo_candidate) = photo_gradient_candidate(&source_rgba, &analysis) {
        trace_candidates.push(photo_candidate);
    }
    if let Some(illustration_candidate) = smooth_illustration_candidate(&source_rgba, &analysis) {
        trace_candidates.push(illustration_candidate);
    }
    let qualities = quality_search_order(
        &analysis,
        options.quality.unwrap_or_default(),
        options.max_iterations.max(1),
    );
    let prepared_metrics = PreparedMetricsInput::from_image(original);
    let gradient_probe_settings =
        qualities.first().copied().map(|quality| adaptive_trace_settings(&analysis, quality));

    let mut best: Option<(String, VectorizationReport, f64)> = None;
    for (quality_index, quality) in qualities.iter().copied().enumerate() {
        let settings = adaptive_trace_settings(&analysis, quality);
        for candidate in &trace_candidates {
            let svg = trace_image(candidate, &settings)?;
            let svg = optimize_svg(&svg, settings.optimizer_precision);
            let metrics = compute_metrics_against(&prepared_metrics, &svg)?;
            let report = VectorizationReport {
                analysis: analysis.clone(),
                settings: settings.clone(),
                quality_preset: quality,
                metrics: metrics.clone(),
            };
            let score = candidate_score(&analysis, &metrics, options.max_file_size);

            if best
                .as_ref()
                .map(|(_, _, best_score)| score > *best_score)
                .unwrap_or(true)
            {
                best = Some((svg, report, score));
            }
        }

        if quality_index == 0 && should_stop_quality_search(&analysis, best.as_ref(), options) {
            break;
        }
    }

    if let Some(settings) = gradient_probe_settings.as_ref() {
        if let Some(svg) = gradient_overlay_candidate(&source_rgba, &analysis, settings)? {
            let svg = optimize_svg(&svg, settings.optimizer_precision);
            let metrics = compute_metrics_against(&prepared_metrics, &svg)?;
            let report = VectorizationReport {
                analysis: analysis.clone(),
                settings: settings.clone(),
                quality_preset: qualities.first().copied().unwrap_or_default(),
                metrics: metrics.clone(),
            };
            let score = candidate_score(&analysis, &metrics, options.max_file_size);

            if best
                .as_ref()
                .map(|(_, _, best_score)| score > *best_score)
                .unwrap_or(true)
            {
                best = Some((svg, report, score));
            }
        }
    }

    let (svg, report, _) = best.context("vectorize produced no candidate")?;
    Ok((svg, report))
}

pub fn write_svg(output_path: &Path, svg: &str) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, svg)
        .with_context(|| format!("unable to write svg {}", output_path.display()))
}

fn trace_image(image: &RgbaImage, settings: &crate::types::TraceSettings) -> Result<String> {
    trace_to_svg(image, settings).map_err(|e| anyhow!("internal vectorizer failed: {e}"))
}

fn flatten_transparency_to_white(image: &RgbaImage) -> RgbaImage {
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

fn trace_candidate_image(
    source: &image::DynamicImage,
    flattened: &image::DynamicImage,
    analysis: &ImageAnalysis,
) -> Result<RgbaImage> {
    if matches!(analysis.image_type, ImageKind::Photo) {
        Ok(flatten_transparency_to_white(&source.to_rgba8()))
    } else if preserve_transparent_logo_icon_source(source, analysis) {
        Ok(source.to_rgba8())
    } else {
        preprocess_image(flattened, analysis, None)
    }
}

fn preserve_transparent_logo_icon_source(
    source: &image::DynamicImage,
    analysis: &ImageAnalysis,
) -> bool {
    has_substantial_transparency(source)
        && matches!(analysis.image_type, ImageKind::Logo | ImageKind::Icon)
        && analysis.effective_colors <= 12
        && analysis.top_10_coverage > 0.92
}

fn has_substantial_transparency(image: &image::DynamicImage) -> bool {
    let rgba = image.to_rgba8();
    let total = (rgba.width() as usize * rgba.height() as usize).max(1);
    let transparent = rgba.pixels().filter(|pixel| pixel[3] < 250).count();
    transparent as f64 / total as f64 > 0.05
}

fn sparse_monochrome_mask_candidate(
    source: &RgbaImage,
    analysis: &ImageAnalysis,
) -> Option<RgbaImage> {
    if !matches!(analysis.image_type, ImageKind::Icon | ImageKind::Logo)
        || analysis.alpha_coverage >= 0.95
    {
        return None;
    }

    let color = detect_sparse_monochrome_color(source)?;
    Some(build_monochrome_alpha_mask(
        source,
        color,
        MONOCHROME_MASK_ALPHA_THRESHOLD,
    ))
}

fn transparent_flat_graphic_candidate(
    source: &RgbaImage,
    analysis: &ImageAnalysis,
) -> Option<RgbaImage> {
    if !matches!(analysis.image_type, ImageKind::Icon | ImageKind::Logo)
        || analysis.alpha_coverage >= 0.98
        || analysis.effective_colors > 96
        || analysis.edge_density > 0.08
    {
        return None;
    }

    let palette_size = match analysis.image_type {
        ImageKind::Logo => analysis.effective_colors.clamp(8, 24),
        ImageKind::Icon => analysis.effective_colors.clamp(12, 32),
        ImageKind::Illustration | ImageKind::Photo => return None,
    };
    let quantized = quantize_image(source, palette_size);
    let softened = if analysis.edge_density < 0.035 {
        image::DynamicImage::ImageRgba8(quantized)
            .blur(0.2)
            .to_rgba8()
    } else {
        quantized
    };
    Some(softened)
}

fn opaque_diagram_candidate(source: &RgbaImage, analysis: &ImageAnalysis) -> Option<RgbaImage> {
    if analysis.alpha_coverage < 0.98
        || analysis.effective_colors > 128
        || analysis.top_50_coverage < 0.80
        || analysis.color_variance > 95.0
        || !(0.006..=0.22).contains(&analysis.edge_density)
    {
        return None;
    }

    let (light_ratio, dark_ratio) = light_dark_coverage(source);
    if light_ratio < 0.35 || dark_ratio < 0.01 {
        return None;
    }

    let palette_size = if analysis.effective_colors <= 16 {
        16
    } else if analysis.effective_colors <= 48 {
        24
    } else {
        32
    };
    Some(quantize_image(source, palette_size))
}

fn transparent_diagram_candidate(
    source: &RgbaImage,
    analysis: &ImageAnalysis,
) -> Option<RgbaImage> {
    if !analysis.is_sparse_diagram_like() || analysis.alpha_coverage >= 0.98 {
        return None;
    }

    let palette_size = if analysis.effective_colors <= 24 {
        16
    } else if analysis.effective_colors <= 64 {
        24
    } else {
        32
    };
    Some(quantize_image(source, palette_size))
}

fn photo_gradient_candidate(source: &RgbaImage, analysis: &ImageAnalysis) -> Option<RgbaImage> {
    if !matches!(analysis.image_type, ImageKind::Photo) || analysis.edge_density > 0.03 {
        return None;
    }

    let longest_side = source.width().max(source.height());
    if longest_side <= 320 {
        return None;
    }

    let smooth_surface = smooth_gradient_factor(analysis);
    let target_longest_side = if smooth_surface >= 0.70 {
        288.0
    } else if analysis.unique_colors > 4_000 && analysis.edge_density < 0.025 {
        384.0
    } else {
        320.0
    };
    let scale = target_longest_side / longest_side as f32;
    let width = ((source.width() as f32) * scale).round().max(1.0) as u32;
    let height = ((source.height() as f32) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(source, width, height, FilterType::Triangle);
    let palette_size = if smooth_surface >= 0.75 {
        6
    } else if analysis.unique_colors > 4_000 || analysis.color_variance > 90.0 {
        8
    } else {
        10
    };
    let candidate = quantize_image(&resized, palette_size);
    Some(
        image::DynamicImage::ImageRgba8(candidate)
            .blur(0.45 + smooth_surface as f32 * 0.25)
            .to_rgba8(),
    )
}

fn smooth_illustration_candidate(
    source: &RgbaImage,
    analysis: &ImageAnalysis,
) -> Option<RgbaImage> {
    if !matches!(analysis.image_type, ImageKind::Illustration)
        || analysis.edge_density > 0.055
        || analysis.unique_colors < 1_024
    {
        return None;
    }

    let longest_side = source.width().max(source.height());
    if longest_side <= 384 {
        return None;
    }

    let smooth_surface = smooth_gradient_factor(analysis);
    let target_longest_side = if smooth_surface >= 0.75 {
        320.0
    } else if smooth_surface >= 0.55 {
        384.0
    } else if analysis.unique_colors > 12_000 {
        448.0
    } else {
        416.0
    };
    let scale = target_longest_side / longest_side as f32;
    let width = ((source.width() as f32) * scale).round().max(1.0) as u32;
    let height = ((source.height() as f32) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(source, width, height, FilterType::Triangle);
    let palette_size = if smooth_surface >= 0.75 {
        8
    } else if smooth_surface >= 0.55 {
        12
    } else if analysis.unique_colors > 20_000 {
        16
    } else {
        18
    };
    let candidate = quantize_image(&resized, palette_size);
    Some(
        image::DynamicImage::ImageRgba8(candidate)
            .blur(0.35 + smooth_surface as f32 * 0.25)
            .to_rgba8(),
    )
}

fn gradient_overlay_candidate(
    source: &RgbaImage,
    analysis: &ImageAnalysis,
    settings: &crate::types::TraceSettings,
) -> Result<Option<String>> {
    if !matches!(analysis.image_type, ImageKind::Illustration | ImageKind::Photo)
        || smooth_gradient_factor(analysis) < 0.55
        || source.width().min(source.height()) < 192
    {
        return Ok(None);
    }

    let surface = match fit_gradient_surface(source, analysis) {
        Some(surface) => surface,
        None => return Ok(None),
    };
    let overlay = decompose_foreground_overlay_with_mask(
        source,
        &surface.gradient,
        surface.residual_tolerance,
        surface.support_mask.as_ref(),
    );
    let overlay = simplify_gradient_overlay(&overlay, analysis);
    let gradient_settings = compact_gradient_trace_settings(settings);
    let overlay_svg = trace_image(&overlay, &gradient_settings)?;
    if let Some(mask) = surface.support_mask.as_ref() {
        let mask_svg = trace_image(mask, &gradient_settings)?;
        Ok(Some(wrap_svg_with_gradient_surface(
            source.width(),
            source.height(),
            &surface.gradient,
            &mask_svg,
            &overlay_svg,
        )))
    } else {
        Ok(Some(wrap_svg_with_gradient_background(
            source.width(),
            source.height(),
            &surface.gradient,
            &overlay_svg,
        )))
    }
}

#[derive(Debug, Clone)]
struct GradientSurface {
    gradient: LinearGradientSpec,
    residual_tolerance: f64,
    support_mask: Option<RgbaImage>,
}

fn fit_gradient_surface(source: &RgbaImage, analysis: &ImageAnalysis) -> Option<GradientSurface> {
    if analysis.alpha_coverage >= 0.98 {
        let gradient = fit_linear_gradient_background(source)?;
        let residual_tolerance = estimate_gradient_residual_tolerance(
            source,
            &gradient,
            &border_sample_mask(source.width(), source.height()),
        )?;
        Some(GradientSurface {
            gradient,
            residual_tolerance,
            support_mask: None,
        })
    } else {
        fit_visible_gradient_surface(source)
    }
}

fn fit_linear_gradient_background(source: &RgbaImage) -> Option<LinearGradientSpec> {
    let width = source.width();
    let height = source.height();
    if width < 2 || height < 2 {
        return None;
    }

    let border_mask = border_sample_mask(source.width(), source.height());
    let luma_plane = fit_plane(source, &border_mask, |pixel| {
        0.299 * f64::from(pixel[0]) + 0.587 * f64::from(pixel[1]) + 0.114 * f64::from(pixel[2])
    })?;
    let direction = normalize2([luma_plane[0], luma_plane[1]])?;

    let corners = [
        project_t(0.0, 0.0, direction),
        project_t(1.0, 0.0, direction),
        project_t(0.0, 1.0, direction),
        project_t(1.0, 1.0, direction),
    ];
    let t_min = corners.into_iter().fold(f64::INFINITY, f64::min);
    let t_max = corners.into_iter().fold(f64::NEG_INFINITY, f64::max);
    if (t_max - t_min).abs() < 1e-6 {
        return None;
    }

    let initial = fit_two_stop_gradient(
        source,
        &border_mask,
        direction,
        t_min,
        t_max,
    )?;
    let initial_tolerance =
        estimate_gradient_residual_tolerance(source, &initial, &border_mask).unwrap_or(4.0);
    let initial_overlay = decompose_foreground_overlay(source, &initial, initial_tolerance);
    let background_pixels = initial_overlay.pixels().filter(|pixel| pixel[3] <= 48).count();
    let background_coverage = background_pixels as f64 / (width as usize * height as usize).max(1) as f64;
    if background_coverage < 0.20 {
        return None;
    }

    Some(refine_gradient_stops(
        source,
        &|x, y| initial_overlay.get_pixel(x, y)[3] <= 48,
        direction,
        t_min,
        t_max,
        5,
    ))
}

fn fit_visible_gradient_surface(source: &RgbaImage) -> Option<GradientSurface> {
    let visible_mask = |x: u32, y: u32| source.get_pixel(x, y)[3] >= 250;
    let visible_pixels = source.pixels().filter(|pixel| pixel[3] >= 16).count();
    let solid_visible_pixels = source.pixels().filter(|pixel| pixel[3] >= 250).count();
    if solid_visible_pixels < (source.width() as usize * source.height() as usize / 8).max(4_096) {
        return None;
    }

    let luma_plane = fit_plane(source, &visible_mask, |pixel| {
        0.299 * f64::from(pixel[0]) + 0.587 * f64::from(pixel[1]) + 0.114 * f64::from(pixel[2])
    })?;
    let direction = normalize2([luma_plane[0], luma_plane[1]])?;
    let corners = [
        project_t(0.0, 0.0, direction),
        project_t(1.0, 0.0, direction),
        project_t(0.0, 1.0, direction),
        project_t(1.0, 1.0, direction),
    ];
    let t_min = corners.into_iter().fold(f64::INFINITY, f64::min);
    let t_max = corners.into_iter().fold(f64::NEG_INFINITY, f64::max);
    if (t_max - t_min).abs() < 1e-6 {
        return None;
    }

    let initial = fit_two_stop_gradient(source, &visible_mask, direction, t_min, t_max)?;
    let support_threshold = (estimate_gradient_residual_percentile(source, &initial, &visible_mask, 0.35)?
        + 3.0)
        .clamp(4.0, 18.0);
    let support_mask = build_gradient_support_mask(source, &initial, support_threshold);
    let support_pixels = support_mask.pixels().filter(|pixel| pixel[3] >= 16).count();
    let support_ratio = support_pixels as f64 / visible_pixels.max(1) as f64;
    if !(0.08..=0.85).contains(&support_ratio) {
        return None;
    }

    let support_mask_fn = |x: u32, y: u32| support_mask.get_pixel(x, y)[3] >= 128;
    let refined = refine_gradient_stops(source, &support_mask_fn, direction, t_min, t_max, 5);
    let refined_tolerance =
        estimate_gradient_residual_tolerance(source, &refined, &support_mask_fn).unwrap_or(4.0);
    Some(GradientSurface {
        gradient: refined,
        residual_tolerance: refined_tolerance,
        support_mask: Some(support_mask),
    })
}

fn fit_two_stop_gradient<F>(
    source: &RgbaImage,
    sample_mask: &F,
    direction: [f64; 2],
    t_min: f64,
    t_max: f64,
) -> Option<LinearGradientSpec>
where
    F: Fn(u32, u32) -> bool,
{
    let mut channels = [[0.0; 2]; 3];
    for channel in 0..3 {
        let (slope, intercept) = fit_channel_on_axis(source, sample_mask, direction, channel)?;
        channels[channel] = [slope, intercept];
    }

    let start = sample_gradient_channels(&channels, t_min);
    let end = sample_gradient_channels(&channels, t_max);
    Some(build_gradient_spec(source.width(), source.height(), direction, t_min, t_max, &[
        (0.0, start),
        (1.0, end),
    ]))
}

fn refine_gradient_stops<F>(
    source: &RgbaImage,
    sample_mask: &F,
    direction: [f64; 2],
    t_min: f64,
    t_max: f64,
    stop_count: usize,
) -> LinearGradientSpec
where
    F: Fn(u32, u32) -> bool,
{
    let mut sums = vec![[0.0; 3]; stop_count];
    let mut counts = vec![0usize; stop_count];

    for y in 0..source.height() {
        for x in 0..source.width() {
            if !sample_mask(x, y) {
                continue;
            }
            let nx = normalized_coord(x, source.width());
            let ny = normalized_coord(y, source.height());
            let t = ((project_t(nx, ny, direction) - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
            let bucket = ((t * (stop_count.saturating_sub(1)) as f64).round() as usize)
                .min(stop_count.saturating_sub(1));
            let pixel = source.get_pixel(x, y);
            for channel in 0..3 {
                sums[bucket][channel] += f64::from(pixel[channel]);
            }
            counts[bucket] += 1;
        }
    }

    let mut stops: Vec<(f64, [u8; 3])> = Vec::with_capacity(stop_count);
    for index in 0..stop_count {
        let offset = if stop_count <= 1 {
            0.0
        } else {
            index as f64 / (stop_count - 1) as f64
        };
        let color = if counts[index] > 0 {
            [
                (sums[index][0] / counts[index] as f64).round().clamp(0.0, 255.0) as u8,
                (sums[index][1] / counts[index] as f64).round().clamp(0.0, 255.0) as u8,
                (sums[index][2] / counts[index] as f64).round().clamp(0.0, 255.0) as u8,
            ]
        } else if let Some(previous) = stops.last() {
            previous.1
        } else {
            let pixel = source.get_pixel(0, 0);
            [pixel[0], pixel[1], pixel[2]]
        };
        stops.push((offset, color));
    }

    build_gradient_spec(source.width(), source.height(), direction, t_min, t_max, &stops)
}

fn build_gradient_spec(
    width: u32,
    height: u32,
    direction: [f64; 2],
    t_min: f64,
    t_max: f64,
    stops: &[(f64, [u8; 3])],
) -> LinearGradientSpec {
    let x1 = direction[0] * t_min * width as f64;
    let y1 = direction[1] * t_min * height as f64;
    let x2 = direction[0] * t_max * width as f64;
    let y2 = direction[1] * t_max * height as f64;

    LinearGradientSpec {
        x1,
        y1,
        x2,
        y2,
        stops: stops
            .iter()
            .map(|(offset, color)| LinearGradientStop {
                offset: *offset,
                color: *color,
            })
            .collect(),
    }
}

fn decompose_foreground_overlay(
    source: &RgbaImage,
    gradient: &LinearGradientSpec,
    residual_tolerance: f64,
) -> RgbaImage {
    decompose_foreground_overlay_with_mask(source, gradient, residual_tolerance, None)
}

fn decompose_foreground_overlay_with_mask(
    source: &RgbaImage,
    gradient: &LinearGradientSpec,
    residual_tolerance: f64,
    support_mask: Option<&RgbaImage>,
) -> RgbaImage {
    let mut overlay = RgbaImage::new(source.width(), source.height());
    for y in 0..source.height() {
        for x in 0..source.width() {
            let actual = source.get_pixel(x, y);
            if actual[3] < 16 {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }
            if let Some(mask) = support_mask {
                if mask.get_pixel(x, y)[3] < 16 {
                    overlay.put_pixel(x, y, *actual);
                    continue;
                }
            }
            let background = sample_gradient_color(gradient, x as f64, y as f64);
            if max_channel_residual([actual[0], actual[1], actual[2]], background) <= residual_tolerance {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }
            let alpha = minimal_unblend_alpha(
                [actual[0], actual[1], actual[2]],
                background,
                residual_tolerance,
            );
            if alpha <= 0.02 {
                overlay.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                continue;
            }
            let foreground = unblend_pixel([actual[0], actual[1], actual[2]], background, alpha);
            overlay.put_pixel(
                x,
                y,
                Rgba([
                    foreground[0],
                    foreground[1],
                    foreground[2],
                    (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }
    overlay
}

fn build_gradient_support_mask(
    source: &RgbaImage,
    gradient: &LinearGradientSpec,
    residual_threshold: f64,
) -> RgbaImage {
    let mut mask = RgbaImage::new(source.width(), source.height());
    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = source.get_pixel(x, y);
            let keep = pixel[3] >= 16
                && max_channel_residual(
                    [pixel[0], pixel[1], pixel[2]],
                    sample_gradient_color(gradient, x as f64, y as f64),
                ) <= residual_threshold;
            mask.put_pixel(
                x,
                y,
                if keep {
                    Rgba([0, 0, 0, pixel[3]])
                } else {
                    Rgba([0, 0, 0, 0])
                },
            );
        }
    }

    let softened = image::DynamicImage::ImageRgba8(mask).blur(1.1).to_rgba8();
    let mut refined = RgbaImage::new(source.width(), source.height());
    for y in 0..source.height() {
        for x in 0..source.width() {
            let source_pixel = source.get_pixel(x, y);
            let alpha = if source_pixel[3] >= 16 && softened.get_pixel(x, y)[3] >= 96 {
                source_pixel[3]
            } else {
                0
            };
            refined.put_pixel(x, y, Rgba([0, 0, 0, alpha]));
        }
    }
    refined
}

fn simplify_gradient_overlay(overlay: &RgbaImage, analysis: &ImageAnalysis) -> RgbaImage {
    let mut cleaned = overlay.clone();
    for pixel in cleaned.pixels_mut() {
        if pixel[3] < 24 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }

    let palette_size = match analysis.image_type {
        ImageKind::Illustration => 12,
        ImageKind::Photo => 16,
        _ => 16,
    };
    quantize_image(&cleaned, palette_size)
}

fn compact_gradient_trace_settings(
    settings: &crate::types::TraceSettings,
) -> crate::types::TraceSettings {
    let mut compact = settings.clone();
    compact.mode = crate::types::TraceMode::Polygon;
    compact.filter_speckle = compact.filter_speckle.max(3);
    compact.color_precision = compact.color_precision.min(5);
    compact.layer_difference = compact.layer_difference.max(20);
    compact.length_threshold += 1.0;
    compact.corner_threshold = compact.corner_threshold.min(35);
    compact.max_iterations = compact.max_iterations.min(12);
    compact.path_precision = compact.path_precision.min(4);
    compact.optimizer_precision = compact.optimizer_precision.min(1);
    compact
}

fn estimate_gradient_residual_tolerance<F>(
    source: &RgbaImage,
    gradient: &LinearGradientSpec,
    sample_mask: &F,
) -> Option<f64>
where
    F: Fn(u32, u32) -> bool,
{
    let residuals = collect_gradient_residuals(source, gradient, sample_mask);
    if residuals.len() < 64 {
        return None;
    }
    let index = ((residuals.len() as f64 - 1.0) * 0.90).round() as usize;
    Some((residuals[index] + 2.0).clamp(3.0, 24.0))
}

fn estimate_gradient_residual_percentile<F>(
    source: &RgbaImage,
    gradient: &LinearGradientSpec,
    sample_mask: &F,
    percentile: f64,
) -> Option<f64>
where
    F: Fn(u32, u32) -> bool,
{
    let residuals = collect_gradient_residuals(source, gradient, sample_mask);
    if residuals.len() < 64 {
        return None;
    }
    let index = ((residuals.len() as f64 - 1.0) * percentile.clamp(0.0, 1.0)).round() as usize;
    residuals.get(index).copied()
}

fn collect_gradient_residuals<F>(
    source: &RgbaImage,
    gradient: &LinearGradientSpec,
    sample_mask: &F,
) -> Vec<f64>
where
    F: Fn(u32, u32) -> bool,
{
    let mut residuals = Vec::new();
    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = source.get_pixel(x, y);
            if pixel[3] < 250 || !sample_mask(x, y) {
                continue;
            }
            let background = sample_gradient_color(gradient, x as f64, y as f64);
            residuals.push(max_channel_residual([pixel[0], pixel[1], pixel[2]], background));
        }
    }
    residuals.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    residuals
}

fn sample_gradient_color(gradient: &LinearGradientSpec, px: f64, py: f64) -> [u8; 3] {
    let dx = gradient.x2 - gradient.x1;
    let dy = gradient.y2 - gradient.y1;
    let denom = dx * dx + dy * dy;
    let t = if denom <= 1e-6 {
        0.0
    } else {
        (((px - gradient.x1) * dx + (py - gradient.y1) * dy) / denom).clamp(0.0, 1.0)
    };

    for window in gradient.stops.windows(2) {
        let left = &window[0];
        let right = &window[1];
        if t <= right.offset {
            let local = ((t - left.offset) / (right.offset - left.offset).max(1e-6)).clamp(0.0, 1.0);
            return [
                lerp_u8(left.color[0], right.color[0], local),
                lerp_u8(left.color[1], right.color[1], local),
                lerp_u8(left.color[2], right.color[2], local),
            ];
        }
    }

    gradient
        .stops
        .last()
        .map(|stop| stop.color)
        .unwrap_or([255, 255, 255])
}

fn fit_channel_on_axis<F>(
    source: &RgbaImage,
    sample_mask: &F,
    direction: [f64; 2],
    channel: usize,
) -> Option<(f64, f64)>
where
    F: Fn(u32, u32) -> bool,
{
    let mut n = 0.0;
    let mut sum_t = 0.0;
    let mut sum_tt = 0.0;
    let mut sum_v = 0.0;
    let mut sum_tv = 0.0;

    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = source.get_pixel(x, y);
            if pixel[3] < 250 || !sample_mask(x, y) {
                continue;
            }
            let t = project_t(
                normalized_coord(x, source.width()),
                normalized_coord(y, source.height()),
                direction,
            );
            let value = f64::from(pixel[channel]);
            n += 1.0;
            sum_t += t;
            sum_tt += t * t;
            sum_v += value;
            sum_tv += t * value;
        }
    }

    let denom = n * sum_tt - sum_t * sum_t;
    if n <= 0.0 || denom.abs() < 1e-9 {
        return None;
    }
    let slope = (n * sum_tv - sum_t * sum_v) / denom;
    let intercept = (sum_v - slope * sum_t) / n;
    Some((slope, intercept))
}

fn fit_plane<F>(
    source: &RgbaImage,
    sample_mask: &F,
    value_of: impl Fn(&Rgba<u8>) -> f64,
) -> Option<[f64; 3]>
where
    F: Fn(u32, u32) -> bool,
{
    let mut matrix = [[0.0; 3]; 3];
    let mut rhs = [0.0; 3];

    for y in 0..source.height() {
        for x in 0..source.width() {
            let pixel = source.get_pixel(x, y);
            if pixel[3] < 250 || !sample_mask(x, y) {
                continue;
            }
            let row = [
                normalized_coord(x, source.width()),
                normalized_coord(y, source.height()),
                1.0,
            ];
            let value = value_of(pixel);
            for i in 0..3 {
                rhs[i] += row[i] * value;
                for j in 0..3 {
                    matrix[i][j] += row[i] * row[j];
                }
            }
        }
    }

    solve_3x3(matrix, rhs)
}

fn border_sample_mask(width: u32, height: u32) -> impl Fn(u32, u32) -> bool {
    let margin = (width.min(height) / 10).max(8);
    move |x, y| {
        x < margin
            || y < margin
            || x >= width.saturating_sub(margin)
            || y >= height.saturating_sub(margin)
    }
}

fn solve_3x3(mut matrix: [[f64; 3]; 3], mut rhs: [f64; 3]) -> Option<[f64; 3]> {
    for pivot in 0..3 {
        let mut best = pivot;
        for row in pivot + 1..3 {
            if matrix[row][pivot].abs() > matrix[best][pivot].abs() {
                best = row;
            }
        }
        if matrix[best][pivot].abs() < 1e-9 {
            return None;
        }
        if best != pivot {
            matrix.swap(best, pivot);
            rhs.swap(best, pivot);
        }
        let scale = matrix[pivot][pivot];
        for col in pivot..3 {
            matrix[pivot][col] /= scale;
        }
        rhs[pivot] /= scale;
        for row in 0..3 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for col in pivot..3 {
                matrix[row][col] -= factor * matrix[pivot][col];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    Some(rhs)
}

fn normalize2(vector: [f64; 2]) -> Option<[f64; 2]> {
    let length = (vector[0] * vector[0] + vector[1] * vector[1]).sqrt();
    if length <= 1e-6 {
        None
    } else {
        Some([vector[0] / length, vector[1] / length])
    }
}

fn normalized_coord(index: u32, len: u32) -> f64 {
    if len <= 1 {
        0.0
    } else {
        index as f64 / (len - 1) as f64
    }
}

fn project_t(x: f64, y: f64, direction: [f64; 2]) -> f64 {
    x * direction[0] + y * direction[1]
}

fn sample_gradient_channels(channels: &[[f64; 2]; 3], t: f64) -> [u8; 3] {
    [
        (channels[0][0] * t + channels[0][1]).round().clamp(0.0, 255.0) as u8,
        (channels[1][0] * t + channels[1][1]).round().clamp(0.0, 255.0) as u8,
        (channels[2][0] * t + channels[2][1]).round().clamp(0.0, 255.0) as u8,
    ]
}

fn minimal_unblend_alpha(actual: [u8; 3], background: [u8; 3], residual_tolerance: f64) -> f64 {
    let mut alpha: f64 = 0.0;
    for channel in 0..3 {
        let actual = f64::from(actual[channel]);
        let background = f64::from(background[channel]);
        let distance = (actual - background).abs();
        if distance <= residual_tolerance {
            continue;
        }
        let effective_distance = distance - residual_tolerance;
        let required = if actual >= background {
            effective_distance / (255.0 - background).max(1.0)
        } else {
            effective_distance / background.max(1.0)
        };
        alpha = alpha.max(required);
    }
    alpha.clamp(0.0, 1.0)
}

fn unblend_pixel(actual: [u8; 3], background: [u8; 3], alpha: f64) -> [u8; 3] {
    if alpha <= 1e-6 {
        return [0, 0, 0];
    }
    let inv = 1.0 - alpha;
    let channel = |index: usize| {
        ((f64::from(actual[index]) - f64::from(background[index]) * inv) / alpha)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [channel(0), channel(1), channel(2)]
}

fn lerp_u8(left: u8, right: u8, t: f64) -> u8 {
    (f64::from(left) + (f64::from(right) - f64::from(left)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn max_channel_residual(actual: [u8; 3], background: [u8; 3]) -> f64 {
    (0..3)
        .map(|channel| (f64::from(actual[channel]) - f64::from(background[channel])).abs())
        .fold(0.0, f64::max)
}

fn quality_search_order(
    analysis: &ImageAnalysis,
    start: QualityPreset,
    max_iterations: usize,
) -> Vec<QualityPreset> {
    let preferred = if prefers_crisp_geometry_first_pass(analysis) {
        vec![
            QualityPreset::Ultra,
            QualityPreset::Quality,
            QualityPreset::Balanced,
            QualityPreset::Figma,
        ]
    } else if prefers_quality_first_pass(analysis) {
        vec![
            QualityPreset::Quality,
            QualityPreset::Balanced,
            QualityPreset::Figma,
            QualityPreset::Ultra,
        ]
    } else {
        match (analysis.image_type, analysis.complexity) {
            (ImageKind::Logo, _) | (ImageKind::Icon, Complexity::Simple | Complexity::Medium) => {
                vec![
                    QualityPreset::Figma,
                    QualityPreset::Balanced,
                    QualityPreset::Quality,
                    QualityPreset::Ultra,
                ]
            }
            (ImageKind::Icon, Complexity::Complex)
            | (ImageKind::Illustration, Complexity::Simple) => {
                vec![
                    QualityPreset::Balanced,
                    QualityPreset::Quality,
                    QualityPreset::Figma,
                    QualityPreset::Ultra,
                ]
            }
            (ImageKind::Illustration, _) | (ImageKind::Photo, Complexity::Simple) => vec![
                QualityPreset::Quality,
                QualityPreset::Balanced,
                QualityPreset::Ultra,
                QualityPreset::Figma,
            ],
            (ImageKind::Photo, _) => vec![
                QualityPreset::Quality,
                QualityPreset::Ultra,
                QualityPreset::Balanced,
                QualityPreset::Figma,
            ],
        }
    };

    let mut order = Vec::new();
    for preset in std::iter::once(start).chain(preferred.into_iter()) {
        if !order.contains(&preset) {
            order.push(preset);
        }
    }
    order.truncate(max_iterations.min(4));
    order
}

fn is_multicolor_flat_graphic(analysis: &ImageAnalysis) -> bool {
    matches!(analysis.image_type, ImageKind::Icon | ImageKind::Logo)
        && analysis.alpha_coverage < 0.85
        && analysis.edge_density < 0.05
        && analysis.effective_colors > 12
}

fn prefers_quality_first_pass(analysis: &ImageAnalysis) -> bool {
    is_multicolor_flat_graphic(analysis) && analysis.top_10_coverage > 0.90
}

fn prefers_crisp_geometry_first_pass(analysis: &ImageAnalysis) -> bool {
    if is_multicolor_flat_graphic(analysis) {
        return false;
    }

    analysis.is_sparse_diagram_like()
        || (analysis.alpha_coverage >= 0.98
            && analysis.effective_colors <= 96
            && analysis.top_50_coverage >= 0.80
            && analysis.color_variance <= 95.0
            && (0.006..=0.22).contains(&analysis.edge_density))
}

fn candidate_score(
    analysis: &ImageAnalysis,
    metrics: &crate::metrics::QualityMetrics,
    max_file_size: usize,
) -> f64 {
    let smooth_gradient = smooth_gradient_factor(analysis);
    let (base_target_size, target_paths) = complexity_budget(analysis, smooth_gradient);
    let target_size = if smooth_gradient > 0.0 {
        base_target_size.max(max_file_size as f64 * (0.9 + smooth_gradient))
    } else {
        base_target_size
    };
    let fidelity_floor = fidelity_floor(analysis);
    let edge_floor = (edge_floor(analysis) - smooth_gradient * 0.08).max(0.65);
    let size_penalty_weight = 0.12 * (1.0 - smooth_gradient * 0.55);
    let path_penalty_weight = 0.18 * (1.0 - smooth_gradient * 0.55);
    let edge_penalty_weight = 0.25 * (1.0 - smooth_gradient * 0.40);
    let oversize_penalty = if metrics.file_size > max_file_size {
        let oversize = (metrics.file_size - max_file_size) as f64 / max_file_size.max(1) as f64;
        if smooth_gradient > 0.0 {
            oversize.powi(2) * (0.06 - smooth_gradient * 0.05)
        } else {
            oversize.ln_1p() * 0.25
        }
    } else {
        0.0
    };
    let size_penalty = excess_penalty(metrics.file_size as f64, target_size) * size_penalty_weight;
    let path_penalty = excess_penalty(metrics.weighted_path_count, target_paths) * path_penalty_weight;
    let fidelity_penalty = (fidelity_floor - metrics.fidelity_score).max(0.0) * 0.45;
    let local_ssim_penalty = (local_ssim_floor(analysis) - metrics.local_ssim_p10).max(0.0) * 0.30;
    let edge_penalty = (edge_floor - metrics.edge_f1).max(0.0) * edge_penalty_weight;
    let gradient_bonus = if metrics.has_gradients
        && smooth_gradient >= 0.55
        && metrics.fidelity_score >= fidelity_floor - 0.015
    {
        0.03 * smooth_gradient
    } else {
        0.0
    };
    let complexity_penalty = excess_penalty(metrics.file_size as f64, target_size * 1.4)
        * 0.10
        * (1.0 - smooth_gradient * 0.35)
        + excess_penalty(metrics.weighted_path_count, target_paths * 1.35)
            * 0.12
            * (1.0 - smooth_gradient * 0.35);

    candidate_reliability_score(metrics)
        + gradient_bonus
        - size_penalty
        - path_penalty
        - oversize_penalty
        - complexity_penalty
        - fidelity_penalty
        - local_ssim_penalty
        - edge_penalty
}

fn should_stop_quality_search(
    analysis: &ImageAnalysis,
    best: Option<&(String, VectorizationReport, f64)>,
    options: &VectorizeOptions,
) -> bool {
    let Some((_, report, _)) = best else {
        return false;
    };
    let metrics = &report.metrics;
    let smooth_gradient = smooth_gradient_factor(analysis);
    if smooth_gradient >= 0.55 {
        return false;
    }

    let (target_size, target_paths) = complexity_budget(analysis, smooth_gradient);
    let relaxed_target_ssim = (options.target_ssim - 0.002).clamp(0.0, 1.0);
    let tight_size_limit = (options.max_file_size as f64 * 0.92).min(target_size * 1.05);
    let tight_path_limit = target_paths * 1.08;

    (matches!(analysis.image_type, ImageKind::Logo | ImageKind::Icon)
        || analysis.is_sparse_diagram_like())
        && metrics.ssim >= relaxed_target_ssim
        && metrics.fidelity_score >= fidelity_floor(analysis) + 0.01
        && metrics.local_ssim_p10 >= local_ssim_floor(analysis)
        && metrics.edge_f1 >= edge_floor(analysis)
        && (metrics.file_size as f64) <= tight_size_limit
        && metrics.weighted_path_count <= tight_path_limit
}

fn candidate_reliability_score(metrics: &crate::metrics::QualityMetrics) -> f64 {
    let weighted = [
        (metrics.fidelity_score, 0.28),
        (metrics.local_ssim_p10, 0.18),
        (metrics.edge_f1, 0.20),
        (metrics.color_similarity, 0.16),
        (metrics.foreground_iou, 0.10),
        (metrics.gradient_similarity, 0.05),
        (metrics.topology_score, 0.03),
    ];
    let total_weight = weighted
        .iter()
        .map(|(_, weight)| *weight)
        .sum::<f64>()
        .max(1.0);
    let sum = weighted
        .iter()
        .map(|(value, weight)| value.clamp(1e-6, 1.0).ln() * *weight)
        .sum::<f64>();
    (sum / total_weight).exp().clamp(0.0, 1.0)
}

fn excess_penalty(actual: f64, target: f64) -> f64 {
    let excess = (actual / target.max(1.0) - 1.0).max(0.0);
    excess.ln_1p()
}

fn complexity_budget(analysis: &ImageAnalysis, smooth_gradient: f64) -> (f64, f64) {
    let base = match (analysis.image_type, analysis.complexity) {
        (ImageKind::Icon, Complexity::Simple) => (2_000.0, 10.0),
        (ImageKind::Icon, Complexity::Medium) => (2_500.0, 16.0),
        (ImageKind::Icon, Complexity::Complex) => (3_500.0, 24.0),
        (ImageKind::Logo, Complexity::Simple) => (3_000.0, 14.0),
        (ImageKind::Logo, Complexity::Medium) => (4_000.0, 20.0),
        (ImageKind::Logo, Complexity::Complex) => (6_000.0, 28.0),
        (ImageKind::Illustration, Complexity::Simple) => (8_000.0, 40.0),
        (ImageKind::Illustration, Complexity::Medium) => (12_000.0, 72.0),
        (ImageKind::Illustration, Complexity::Complex) => (16_000.0, 96.0),
        (ImageKind::Photo, Complexity::Simple) => (16_000.0, 48.0),
        (ImageKind::Photo, Complexity::Medium) => (24_000.0, 96.0),
        (ImageKind::Photo, Complexity::Complex) => (32_000.0, 144.0),
    };

    let multicolor_flat_graphic = matches!(analysis.image_type, ImageKind::Icon | ImageKind::Logo)
        && analysis.alpha_coverage < 0.85
        && analysis.edge_density < 0.05
        && analysis.effective_colors > 12;
    if multicolor_flat_graphic {
        let detail_factor = ((analysis.effective_colors as f64 - 12.0) / 36.0).clamp(0.0, 1.0);
        let size_multiplier = 1.0 + detail_factor * 6.0;
        let path_multiplier = 1.0 + detail_factor * 1.5;
        return (base.0 * size_multiplier, base.1 * path_multiplier);
    }

    if matches!(
        analysis.image_type,
        ImageKind::Illustration | ImageKind::Photo
    ) {
        let size_multiplier = 1.0 + smooth_gradient * 2.2;
        let path_multiplier = 1.0 + smooth_gradient * 2.6;
        (base.0 * size_multiplier, base.1 * path_multiplier)
    } else {
        base
    }
}

fn smooth_gradient_factor(analysis: &ImageAnalysis) -> f64 {
    if !matches!(
        analysis.image_type,
        ImageKind::Illustration | ImageKind::Photo
    ) {
        return 0.0;
    }

    let smoothness = ((0.05 - analysis.edge_density) / 0.03).clamp(0.0, 1.0);
    let richness = ((analysis.unique_colors as f64 - 1_500.0) / 6_000.0).clamp(0.0, 1.0);
    let coverage = ((1.0 - analysis.top_10_coverage) / 0.35).clamp(0.0, 1.0);

    (smoothness * 0.45 + richness * 0.35 + coverage * 0.20).clamp(0.0, 1.0)
}

fn fidelity_floor(analysis: &ImageAnalysis) -> f64 {
    match (analysis.image_type, analysis.complexity) {
        (ImageKind::Logo, Complexity::Simple) => 0.92,
        (ImageKind::Logo, _) => 0.90,
        (ImageKind::Icon, Complexity::Simple | Complexity::Medium) => 0.89,
        (ImageKind::Icon, Complexity::Complex) => 0.87,
        (ImageKind::Illustration, Complexity::Simple) => 0.87,
        (ImageKind::Illustration, Complexity::Medium) => 0.85,
        (ImageKind::Illustration, Complexity::Complex) => 0.83,
        (ImageKind::Photo, Complexity::Simple) => 0.82,
        (ImageKind::Photo, Complexity::Medium) => 0.80,
        (ImageKind::Photo, Complexity::Complex) => 0.78,
    }
}

fn edge_floor(analysis: &ImageAnalysis) -> f64 {
    match analysis.image_type {
        ImageKind::Logo => 0.90,
        ImageKind::Icon => 0.88,
        ImageKind::Illustration => 0.84,
        ImageKind::Photo => 0.78,
    }
}

fn local_ssim_floor(analysis: &ImageAnalysis) -> f64 {
    match analysis.image_type {
        ImageKind::Logo => 0.88,
        ImageKind::Icon => 0.86,
        ImageKind::Illustration => 0.80,
        ImageKind::Photo => 0.76,
    }
}

fn light_dark_coverage(source: &RgbaImage) -> (f64, f64) {
    let total = (source.width() as usize * source.height() as usize).max(1) as f64;
    let mut light = 0usize;
    let mut dark = 0usize;
    for pixel in source.pixels() {
        let luminance =
            0.299 * f64::from(pixel[0]) + 0.587 * f64::from(pixel[1]) + 0.114 * f64::from(pixel[2]);
        if luminance >= 245.0 {
            light += 1;
        }
        if luminance <= 80.0 {
            dark += 1;
        }
    }
    (light as f64 / total, dark as f64 / total)
}

pub fn vectorize_logo(
    input_path: &Path,
    target_size_kb: usize,
) -> Result<(String, VectorizationReport)> {
    vectorize(
        input_path,
        &VectorizeOptions {
            target_ssim: 0.98,
            max_file_size: target_size_kb * 1024,
            max_iterations: 4,
            quality: Some(QualityPreset::Ultra),
        },
    )
}

pub fn vectorize_icon(input_path: &Path) -> Result<(String, VectorizationReport)> {
    vectorize(
        input_path,
        &VectorizeOptions {
            target_ssim: 0.998,
            max_file_size: 100_000,
            max_iterations: 4,
            quality: Some(QualityPreset::Ultra),
        },
    )
}

#[cfg(test)]
mod tests {
    use image::DynamicImage;
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    use crate::metrics::QualityMetrics;
    use crate::preprocess::adaptive_trace_settings;
    use crate::types::{Complexity, ImageAnalysis, ImageKind, VectorizationReport};

    use super::{
        gradient_overlay_candidate, quality_search_order, trace_candidate_image, vectorize,
        QualityPreset, VectorizeOptions,
    };

    #[test]
    fn quality_search_starts_at_requested_preset_and_climbs() {
        let icon_analysis = ImageAnalysis {
            width: 24,
            height: 24,
            unique_colors: 8,
            effective_colors: 4,
            top_10_coverage: 0.98,
            top_50_coverage: 1.0,
            color_variance: 20.0,
            edge_density: 0.1,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };
        let photo_analysis = ImageAnalysis {
            width: 640,
            height: 480,
            unique_colors: 4_096,
            effective_colors: 512,
            top_10_coverage: 0.18,
            top_50_coverage: 0.28,
            color_variance: 92.0,
            edge_density: 0.24,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Photo,
            complexity: Complexity::Complex,
        };
        assert_eq!(
            quality_search_order(&icon_analysis, QualityPreset::Figma, 4),
            vec![
                QualityPreset::Figma,
                QualityPreset::Ultra,
                QualityPreset::Quality,
                QualityPreset::Balanced
            ]
        );
        assert_eq!(
            quality_search_order(&icon_analysis, QualityPreset::Balanced, 3),
            vec![
                QualityPreset::Balanced,
                QualityPreset::Ultra,
                QualityPreset::Quality
            ]
        );
        assert_eq!(
            quality_search_order(&photo_analysis, QualityPreset::Quality, 4),
            vec![
                QualityPreset::Quality,
                QualityPreset::Ultra,
                QualityPreset::Balanced,
                QualityPreset::Figma
            ]
        );
    }

    #[test]
    fn quality_search_promotes_quality_for_concentrated_flat_transparent_graphics() {
        let analysis = ImageAnalysis {
            width: 512,
            height: 512,
            unique_colors: 256,
            effective_colors: 40,
            top_10_coverage: 0.98,
            top_50_coverage: 1.0,
            color_variance: 55.0,
            edge_density: 0.02,
            alpha_coverage: 0.5,
            dominant_colors: vec!["#4285F4".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };

        assert_eq!(
            quality_search_order(&analysis, QualityPreset::Figma, 2),
            vec![QualityPreset::Figma, QualityPreset::Quality]
        );
    }

    #[test]
    fn quality_search_keeps_balanced_second_for_gradient_heavy_transparent_icons() {
        let analysis = ImageAnalysis {
            width: 512,
            height: 512,
            unique_colors: 4_096,
            effective_colors: 27,
            top_10_coverage: 0.10,
            top_50_coverage: 0.30,
            color_variance: 18.0,
            edge_density: 0.02,
            alpha_coverage: 0.4,
            dominant_colors: vec!["#FAA319".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };

        assert_eq!(
            quality_search_order(&analysis, QualityPreset::Figma, 2),
            vec![QualityPreset::Figma, QualityPreset::Balanced]
        );
    }

    #[test]
    fn quality_search_promotes_ultra_for_opaque_diagram_like_assets() {
        let analysis = ImageAnalysis {
            width: 1024,
            height: 768,
            unique_colors: 96,
            effective_colors: 44,
            top_10_coverage: 0.63,
            top_50_coverage: 0.94,
            color_variance: 38.0,
            edge_density: 0.09,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Illustration,
            complexity: Complexity::Medium,
        };

        assert_eq!(
            quality_search_order(&analysis, QualityPreset::Figma, 3),
            vec![
                QualityPreset::Figma,
                QualityPreset::Ultra,
                QualityPreset::Quality
            ]
        );
    }

    #[test]
    fn quality_search_promotes_ultra_for_sparse_transparent_diagram_like_assets() {
        let analysis = ImageAnalysis {
            width: 1024,
            height: 768,
            unique_colors: 192,
            effective_colors: 56,
            top_10_coverage: 0.84,
            top_50_coverage: 0.97,
            color_variance: 74.0,
            edge_density: 0.09,
            alpha_coverage: 0.18,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Illustration,
            complexity: Complexity::Medium,
        };

        assert_eq!(
            quality_search_order(&analysis, QualityPreset::Figma, 3),
            vec![
                QualityPreset::Figma,
                QualityPreset::Ultra,
                QualityPreset::Quality
            ]
        );
    }

    #[test]
    fn vectorize_handles_transparent_black_logo_without_filling_background() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("transparent_logo.png");
        let mut image = RgbaImage::new(24, 24);
        for y in 0..24 {
            for x in 0..24 {
                let pixel = if x > 5 && x < 18 && y > 5 && y < 18 {
                    Rgba([0, 0, 0, 255])
                } else {
                    Rgba([0, 0, 0, 0])
                };
                image.put_pixel(x, y, pixel);
            }
        }
        image.save(&path).unwrap();

        let (_, report) = vectorize(
            &path,
            &VectorizeOptions {
                target_ssim: 0.9,
                max_file_size: 50_000,
                max_iterations: 4,
                quality: Some(QualityPreset::Figma),
            },
        )
        .unwrap();

        assert!(report.metrics.ssim > 0.8);
        assert!(report.metrics.path_count >= 1);
    }

    #[test]
    fn candidate_score_prefers_compact_icon_when_quality_gain_is_small() {
        let analysis = ImageAnalysis {
            width: 24,
            height: 24,
            unique_colors: 24,
            effective_colors: 8,
            top_10_coverage: 0.94,
            top_50_coverage: 1.0,
            color_variance: 45.0,
            edge_density: 0.2,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };
        let compact = QualityMetrics {
            ssim: 0.88,
            ssim_perceptual: 0.92,
            local_ssim_p10: 0.84,
            local_ssim_min: 0.82,
            gradient_similarity: 0.90,
            edge_similarity: 0.95,
            edge_precision: 0.96,
            edge_recall: 0.94,
            edge_f1: 0.95,
            foreground_iou: 0.93,
            color_similarity: 0.78,
            fidelity_score: 0.89,
            delta_e: 9.0,
            topology_score: 1.0,
            psnr: 16.0,
            mae: 20.0,
            file_size: 2_000,
            path_count: 8,
            primitive_count: 0,
            weighted_path_count: 8.0,
            has_gradients: false,
        };
        let bloated = QualityMetrics {
            ssim: 0.91,
            ssim_perceptual: 0.94,
            local_ssim_p10: 0.85,
            local_ssim_min: 0.83,
            gradient_similarity: 0.92,
            edge_similarity: 0.96,
            edge_precision: 0.97,
            edge_recall: 0.95,
            edge_f1: 0.96,
            foreground_iou: 0.94,
            color_similarity: 0.80,
            fidelity_score: 0.90,
            delta_e: 8.0,
            topology_score: 1.0,
            psnr: 17.0,
            mae: 15.0,
            file_size: 10_000,
            path_count: 120,
            primitive_count: 0,
            weighted_path_count: 120.0,
            has_gradients: false,
        };

        assert!(
            super::candidate_score(&analysis, &compact, 100_000)
                > super::candidate_score(&analysis, &bloated, 100_000)
        );
    }

    #[test]
    fn candidate_score_allows_reasonable_growth_for_smooth_gradient_scenes() {
        let analysis = ImageAnalysis {
            width: 989,
            height: 1024,
            unique_colors: 24_514,
            effective_colors: 512,
            top_10_coverage: 0.32,
            top_50_coverage: 0.56,
            color_variance: 84.0,
            edge_density: 0.011,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#FF6600".to_string()],
            image_type: ImageKind::Illustration,
            complexity: Complexity::Medium,
        };
        let compact = QualityMetrics {
            ssim: 0.9948,
            ssim_perceptual: 0.9976,
            local_ssim_p10: 0.8730,
            local_ssim_min: 0.8120,
            gradient_similarity: 0.9190,
            edge_similarity: 0.6923,
            edge_precision: 0.8195,
            edge_recall: 0.8168,
            edge_f1: 0.8181,
            foreground_iou: 0.9958,
            color_similarity: 0.9288,
            fidelity_score: 0.9023,
            delta_e: 2.84,
            topology_score: 0.5,
            psnr: 29.46,
            mae: 3.27,
            file_size: 132_618,
            path_count: 85,
            primitive_count: 0,
            weighted_path_count: 85.0,
            has_gradients: false,
        };
        let richer = QualityMetrics {
            ssim: 0.9954,
            ssim_perceptual: 0.9979,
            local_ssim_p10: 0.9010,
            local_ssim_min: 0.8540,
            gradient_similarity: 0.9380,
            edge_similarity: 0.7074,
            edge_precision: 0.8294,
            edge_recall: 0.8278,
            edge_f1: 0.8286,
            foreground_iou: 0.9964,
            color_similarity: 0.9335,
            fidelity_score: 0.9085,
            delta_e: 2.66,
            topology_score: 0.5,
            psnr: 29.97,
            mae: 3.04,
            file_size: 183_478,
            path_count: 123,
            primitive_count: 0,
            weighted_path_count: 123.0,
            has_gradients: false,
        };

        assert!(
            super::candidate_score(&analysis, &richer, 100_000)
                > super::candidate_score(&analysis, &compact, 100_000)
        );
    }

    #[test]
    fn candidate_score_rejects_color_collapsed_logo_variants() {
        let analysis = ImageAnalysis {
            width: 512,
            height: 512,
            unique_colors: 2_048,
            effective_colors: 28,
            top_10_coverage: 0.72,
            top_50_coverage: 0.91,
            color_variance: 44.0,
            edge_density: 0.02,
            alpha_coverage: 0.62,
            dominant_colors: vec!["#0099FF".to_string()],
            image_type: ImageKind::Logo,
            complexity: Complexity::Medium,
        };
        let color_faithful = QualityMetrics {
            ssim: 0.90,
            ssim_perceptual: 0.93,
            local_ssim_p10: 0.88,
            local_ssim_min: 0.85,
            gradient_similarity: 0.89,
            edge_similarity: 0.92,
            edge_precision: 0.93,
            edge_recall: 0.91,
            edge_f1: 0.92,
            foreground_iou: 0.95,
            color_similarity: 0.91,
            fidelity_score: 0.91,
            delta_e: 3.0,
            topology_score: 0.98,
            psnr: 22.0,
            mae: 9.0,
            file_size: 7_200,
            path_count: 16,
            primitive_count: 0,
            weighted_path_count: 16.0,
            has_gradients: false,
        };
        let collapsed = QualityMetrics {
            ssim: 0.90,
            ssim_perceptual: 0.93,
            local_ssim_p10: 0.61,
            local_ssim_min: 0.42,
            gradient_similarity: 0.88,
            edge_similarity: 0.93,
            edge_precision: 0.93,
            edge_recall: 0.92,
            edge_f1: 0.92,
            foreground_iou: 0.95,
            color_similarity: 0.24,
            fidelity_score: 0.78,
            delta_e: 24.0,
            topology_score: 0.98,
            psnr: 18.0,
            mae: 18.0,
            file_size: 1_800,
            path_count: 2,
            primitive_count: 0,
            weighted_path_count: 2.0,
            has_gradients: false,
        };

        assert!(
            super::candidate_score(&analysis, &color_faithful, 100_000)
                > super::candidate_score(&analysis, &collapsed, 100_000)
        );
    }

    #[test]
    fn multicolor_transparent_icons_use_flattened_preprocess_candidate() {
        let mut rgba = RgbaImage::new(32, 32);
        for y in 8..24 {
            for x in 8..24 {
                let color = if x < 16 {
                    Rgba([66, 133, 244, 180])
                } else {
                    Rgba([234, 67, 53, 180])
                };
                rgba.put_pixel(x, y, color);
            }
        }

        let source = DynamicImage::ImageRgba8(rgba.clone());
        let flattened = DynamicImage::ImageRgba8(super::flatten_transparency_to_white(&rgba));
        let analysis = ImageAnalysis {
            width: 32,
            height: 32,
            unique_colors: 40,
            effective_colors: 16,
            top_10_coverage: 0.96,
            top_50_coverage: 1.0,
            color_variance: 60.0,
            edge_density: 0.02,
            alpha_coverage: 0.25,
            dominant_colors: vec!["#4285F4".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };

        let candidate = trace_candidate_image(&source, &flattened, &analysis).unwrap();
        assert!(candidate.pixels().all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn gradient_overlay_candidate_emits_surface_gradient_for_transparent_subject() {
        let width = 256;
        let height = 256;
        let mut image = RgbaImage::new(width, height);
        for y in 32..224 {
            for x in 24..232 {
                let t = (x - 24) as f64 / 208.0;
                let value = (90.0 + t * 90.0).round() as u8;
                image.put_pixel(x, y, Rgba([value, value, value, 255]));
            }
        }
        for y in 96..152 {
            for x in 112..180 {
                image.put_pixel(x, y, Rgba([35, 35, 35, 255]));
            }
        }

        let analysis = ImageAnalysis {
            width,
            height,
            unique_colors: 6_000,
            effective_colors: 96,
            top_10_coverage: 0.28,
            top_50_coverage: 0.60,
            color_variance: 56.0,
            edge_density: 0.012,
            alpha_coverage: 0.63,
            dominant_colors: vec!["#808080".to_string()],
            image_type: ImageKind::Illustration,
            complexity: Complexity::Medium,
        };
        let settings = adaptive_trace_settings(&analysis, QualityPreset::Balanced);

        let svg = gradient_overlay_candidate(&image, &analysis, &settings)
            .unwrap()
            .expect("gradient surface candidate");

        assert!(svg.contains("<linearGradient"));
        assert!(svg.contains("surfaceGradient"));
        assert!(!svg.contains(r#"<rect width="256" height="256" fill="url(#bgGradient)"/>"#));
    }

    #[test]
    fn quality_search_can_stop_early_for_high_quality_icon_result() {
        let analysis = ImageAnalysis {
            width: 128,
            height: 128,
            unique_colors: 24,
            effective_colors: 8,
            top_10_coverage: 0.96,
            top_50_coverage: 1.0,
            color_variance: 34.0,
            edge_density: 0.03,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };
        let metrics = QualityMetrics {
            ssim: 0.997,
            ssim_perceptual: 0.998,
            local_ssim_p10: 0.93,
            local_ssim_min: 0.90,
            gradient_similarity: 0.95,
            edge_similarity: 0.95,
            edge_precision: 0.96,
            edge_recall: 0.95,
            edge_f1: 0.95,
            foreground_iou: 0.97,
            color_similarity: 0.96,
            fidelity_score: 0.95,
            delta_e: 1.5,
            topology_score: 1.0,
            psnr: 34.0,
            mae: 1.0,
            file_size: 2_200,
            path_count: 6,
            primitive_count: 2,
            weighted_path_count: 6.7,
            has_gradients: false,
        };
        let report = VectorizationReport {
            analysis: analysis.clone(),
            settings: adaptive_trace_settings(&analysis, QualityPreset::Figma),
            quality_preset: QualityPreset::Figma,
            metrics,
        };

        assert!(super::should_stop_quality_search(
            &analysis,
            Some(&(String::new(), report, 0.95)),
            &VectorizeOptions {
                target_ssim: 0.998,
                max_file_size: 100_000,
                max_iterations: 2,
                quality: Some(QualityPreset::Figma),
            },
        ));
    }

    #[test]
    fn quality_search_keeps_searching_for_smooth_gradient_scene() {
        let analysis = ImageAnalysis {
            width: 1024,
            height: 768,
            unique_colors: 18_000,
            effective_colors: 512,
            top_10_coverage: 0.22,
            top_50_coverage: 0.49,
            color_variance: 78.0,
            edge_density: 0.012,
            alpha_coverage: 1.0,
            dominant_colors: vec!["#808080".to_string()],
            image_type: ImageKind::Illustration,
            complexity: Complexity::Medium,
        };
        let metrics = QualityMetrics {
            ssim: 0.996,
            ssim_perceptual: 0.997,
            local_ssim_p10: 0.92,
            local_ssim_min: 0.89,
            gradient_similarity: 0.96,
            edge_similarity: 0.89,
            edge_precision: 0.90,
            edge_recall: 0.89,
            edge_f1: 0.89,
            foreground_iou: 0.98,
            color_similarity: 0.95,
            fidelity_score: 0.93,
            delta_e: 1.8,
            topology_score: 0.9,
            psnr: 31.0,
            mae: 1.4,
            file_size: 18_000,
            path_count: 36,
            primitive_count: 0,
            weighted_path_count: 36.0,
            has_gradients: false,
        };
        let report = VectorizationReport {
            analysis: analysis.clone(),
            settings: adaptive_trace_settings(&analysis, QualityPreset::Figma),
            quality_preset: QualityPreset::Figma,
            metrics,
        };

        assert!(!super::should_stop_quality_search(
            &analysis,
            Some(&(String::new(), report, 0.93)),
            &VectorizeOptions {
                target_ssim: 0.998,
                max_file_size: 100_000,
                max_iterations: 2,
                quality: Some(QualityPreset::Figma),
            },
        ));
    }

}
