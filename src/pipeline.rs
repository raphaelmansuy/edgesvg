use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::RgbaImage;
use vtracer::ColorImage;

use crate::analysis::analyze_image;
use crate::metrics::{compute_metrics, QualityMetrics};
use crate::preprocess::{
    adaptive_trace_settings, count_unique_colors, preprocess_image, quantize_image,
    to_vtracer_config,
};
use crate::svg::optimize_svg;
use crate::types::{QualityPreset, VectorizationReport};

#[derive(Debug, Clone)]
pub struct VectorizeOptions {
    pub target_ssim: f64,
    pub max_file_size: usize,
    pub max_iterations: usize,
    pub quality: Option<QualityPreset>,
}

impl Default for VectorizeOptions {
    fn default() -> Self {
        Self {
            target_ssim: 0.92,
            max_file_size: 100_000,
            max_iterations: 5,
            quality: None,
        }
    }
}

pub fn vectorize(
    input_path: &Path,
    options: &VectorizeOptions,
) -> Result<(String, VectorizationReport)> {
    let original = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    let analysis = analyze_image(&original);

    let mut candidate = preprocess_image(&original, &analysis, None)?;
    let ladder = if let Some(quality) = options.quality {
        vec![quality; options.max_iterations.max(1)]
    } else {
        (0..options.max_iterations.max(1))
            .map(|index| {
                QualityPreset::ordered_for_iterations()
                    .get(index)
                    .copied()
                    .unwrap_or(QualityPreset::Quality)
            })
            .collect::<Vec<_>>()
    };

    let mut best: Option<(String, VectorizationReport, f64)> = None;

    for (index, quality) in ladder.into_iter().enumerate() {
        let settings = adaptive_trace_settings(&analysis, quality);
        let svg = trace_image(&candidate, &settings)?;
        let svg = optimize_svg(&svg, settings.path_precision);
        let metrics = compute_metrics(&original, &svg)?;
        let score = score_metrics(&metrics, options.max_file_size);

        let report = VectorizationReport {
            analysis: analysis.clone(),
            settings,
            quality_preset: quality,
            metrics: metrics.clone(),
        };

        if best
            .as_ref()
            .map(|(_, _, best_score)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((svg.clone(), report.clone(), score));
        }

        if metrics.ssim >= options.target_ssim && metrics.file_size <= options.max_file_size {
            return Ok((svg, report));
        }

        if metrics.file_size > options.max_file_size && index + 1 < options.max_iterations {
            let current_colors = count_unique_colors(&candidate);
            let next_colors = ((current_colors as f64) * 0.7).round().max(4.0) as usize;
            candidate = quantize_image(&candidate, next_colors);
        }
    }

    best.map(|(svg, report, _)| (svg, report))
        .ok_or_else(|| anyhow!("vectorization did not produce a candidate"))
}

pub fn write_svg(output_path: &Path, svg: &str) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, svg)
        .with_context(|| format!("unable to write svg {}", output_path.display()))
}

fn trace_image(image: &RgbaImage, settings: &crate::types::TraceSettings) -> Result<String> {
    let color_image = ColorImage {
        pixels: image.clone().into_raw(),
        width: image.width() as usize,
        height: image.height() as usize,
    };
    let config = to_vtracer_config(settings);
    let svg = vtracer::convert(color_image, config).map_err(|e| anyhow!("vtracer failed: {e}"))?;
    Ok(svg.to_string())
}

fn score_metrics(metrics: &QualityMetrics, max_file_size: usize) -> f64 {
    let size_score = if metrics.file_size < max_file_size * 3 {
        (1.0 - metrics.file_size as f64 / max_file_size as f64).max(0.0)
    } else {
        -1.0
    };
    metrics.ssim * 0.7 + size_score * 0.3
}

pub fn vectorize_logo(
    input_path: &Path,
    target_size_kb: usize,
) -> Result<(String, VectorizationReport)> {
    vectorize(
        input_path,
        &VectorizeOptions {
            target_ssim: 0.90,
            max_file_size: target_size_kb * 1024,
            max_iterations: 5,
            quality: None,
        },
    )
}

pub fn vectorize_icon(input_path: &Path) -> Result<(String, VectorizationReport)> {
    vectorize(
        input_path,
        &VectorizeOptions {
            target_ssim: 0.92,
            max_file_size: 100_000,
            max_iterations: 4,
            quality: None,
        },
    )
}
