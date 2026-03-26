use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::RgbaImage;
use vtracer::ColorImage;

use crate::analysis::analyze_image;
use crate::metrics::compute_metrics;
use crate::preprocess::{to_vtracer_config, trace_settings_for_preset};
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
            target_ssim: 0.998,
            max_file_size: 100_000,
            max_iterations: 1,
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
    let analysis = analyze_image(&original);
    let quality = options.quality.unwrap_or_default();
    let settings = trace_settings_for_preset(quality);
    let svg = trace_image(&original.to_rgba8(), &settings)?;
    let svg = optimize_svg(&svg, settings.optimizer_precision);
    let metrics = compute_metrics(&original, &svg)?;

    let _ = (
        options.target_ssim,
        options.max_file_size,
        options.max_iterations,
    );

    Ok((
        svg,
        VectorizationReport {
            analysis,
            settings,
            quality_preset: quality,
            metrics,
        },
    ))
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

pub fn vectorize_logo(
    input_path: &Path,
    target_size_kb: usize,
) -> Result<(String, VectorizationReport)> {
    vectorize(
        input_path,
        &VectorizeOptions {
            target_ssim: 0.98,
            max_file_size: target_size_kb * 1024,
            max_iterations: 1,
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
            max_iterations: 1,
            quality: Some(QualityPreset::Ultra),
        },
    )
}
