use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::analysis::analyze_image;
use crate::metrics::compute_metrics;
use crate::preprocess::{
    adaptive_trace_settings, build_monochrome_alpha_mask, detect_sparse_monochrome_color,
    preprocess_image, MONOCHROME_MASK_ALPHA_THRESHOLD,
};
use crate::svg::optimize_svg;
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
    let qualities = quality_search_order(
        &analysis,
        options.quality.unwrap_or_default(),
        options.max_iterations.max(1),
    );

    let mut best: Option<(String, VectorizationReport, f64)> = None;
    for quality in qualities {
        let settings = adaptive_trace_settings(&analysis, quality);
        for candidate in &trace_candidates {
            let svg = trace_image(candidate, &settings)?;
            let svg = optimize_svg(&svg, settings.optimizer_precision);
            let metrics = compute_metrics(original, &svg)?;
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
    } else if has_substantial_transparency(source)
        && matches!(analysis.image_type, ImageKind::Logo | ImageKind::Icon)
    {
        Ok(source.to_rgba8())
    } else {
        preprocess_image(flattened, analysis, None)
    }
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

fn quality_search_order(
    analysis: &ImageAnalysis,
    start: QualityPreset,
    max_iterations: usize,
) -> Vec<QualityPreset> {
    let preferred = match (analysis.image_type, analysis.complexity) {
        (ImageKind::Logo, _) | (ImageKind::Icon, Complexity::Simple | Complexity::Medium) => {
            vec![
                QualityPreset::Figma,
                QualityPreset::Balanced,
                QualityPreset::Quality,
                QualityPreset::Ultra,
            ]
        }
        (ImageKind::Icon, Complexity::Complex) | (ImageKind::Illustration, Complexity::Simple) => {
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

fn candidate_score(
    analysis: &ImageAnalysis,
    metrics: &crate::metrics::QualityMetrics,
    max_file_size: usize,
) -> f64 {
    let (target_size, target_paths) = complexity_budget(analysis);
    let fidelity_floor = fidelity_floor(analysis);
    let edge_floor = edge_floor(analysis);
    let oversize_penalty = if metrics.file_size > max_file_size {
        let oversize = (metrics.file_size - max_file_size) as f64 / max_file_size.max(1) as f64;
        oversize.ln_1p() * 0.25
    } else {
        0.0
    };
    let size_penalty = excess_penalty(metrics.file_size as f64, target_size) * 0.12;
    let path_penalty = excess_penalty(metrics.path_count as f64, target_paths) * 0.18;
    let fidelity_penalty = (fidelity_floor - metrics.fidelity_score).max(0.0) * 0.45;
    let edge_penalty = (edge_floor - metrics.edge_f1).max(0.0) * 0.25;

    metrics.fidelity_score
        - size_penalty
        - path_penalty
        - oversize_penalty
        - fidelity_penalty
        - edge_penalty
}

fn excess_penalty(actual: f64, target: f64) -> f64 {
    let excess = (actual / target.max(1.0) - 1.0).max(0.0);
    excess.ln_1p()
}

fn complexity_budget(analysis: &ImageAnalysis) -> (f64, f64) {
    match (analysis.image_type, analysis.complexity) {
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
    }
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
    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    use crate::metrics::QualityMetrics;
    use crate::types::{Complexity, ImageAnalysis, ImageKind};

    use super::{quality_search_order, vectorize, QualityPreset, VectorizeOptions};

    #[test]
    fn quality_search_starts_at_requested_preset_and_climbs() {
        let icon_analysis = ImageAnalysis {
            width: 24,
            height: 24,
            unique_colors: 8,
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
                QualityPreset::Balanced,
                QualityPreset::Quality,
                QualityPreset::Ultra
            ]
        );
        assert_eq!(
            quality_search_order(&icon_analysis, QualityPreset::Balanced, 3),
            vec![
                QualityPreset::Balanced,
                QualityPreset::Figma,
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
        };
        let bloated = QualityMetrics {
            ssim: 0.91,
            ssim_perceptual: 0.94,
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
        };

        assert!(
            super::candidate_score(&analysis, &compact, 100_000)
                > super::candidate_score(&analysis, &bloated, 100_000)
        );
    }
}
