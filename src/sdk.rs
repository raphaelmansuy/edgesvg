use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use image::codecs::png::PngEncoder;
use image::{ColorType, DynamicImage, ImageEncoder};
use serde::{Deserialize, Serialize};

use crate::benchmark::{benchmark_directory, benchmark_golden_data, BenchmarkReport};
use crate::highlevel::{
    determine_auto_mode_image, vectorize_auto_image, vectorize_logo_premium_image,
    vectorize_optimal_image, vectorize_premium_image, vectorize_smart_image, AutoDecision,
};
use crate::metrics::{compute_metrics, render_svg_to_image, QualityMetrics};
use crate::pipeline::{vectorize_image, VectorizeOptions};
use crate::svg::optimize_svg;
use crate::types::{
    ImageAnalysis, LogoQualityPreset, QualityPreset, VectorizationReport, VectorizeMethod,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VectorizeRequest {
    pub method: VectorizeMethod,
    pub target_ssim: f64,
    pub max_file_size: usize,
    pub max_iterations: usize,
    pub quality: QualityPreset,
    pub logo_quality: Option<LogoQualityPreset>,
    pub colors: Option<usize>,
}

impl Default for VectorizeRequest {
    fn default() -> Self {
        Self {
            method: VectorizeMethod::Hifi,
            target_ssim: 0.998,
            max_file_size: 100_000,
            max_iterations: 4,
            quality: QualityPreset::Ultra,
            logo_quality: None,
            colors: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorizeResponse {
    pub svg: String,
    pub report: VectorizationReport,
    pub requested_method: VectorizeMethod,
    pub effective_method: VectorizeMethod,
    pub fallback_from: Option<VectorizeMethod>,
    pub decision: Option<AutoDecision>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeResponse {
    pub analysis: ImageAnalysis,
    pub decision: AutoDecision,
}

#[derive(Debug, Clone, Serialize)]
pub struct InfoResponse {
    pub file_size_bytes: Option<u64>,
    pub format: Option<String>,
    pub channels: u8,
    pub color_mode: String,
    pub analysis: ImageAnalysis,
    pub recommended_method: VectorizeMethod,
    pub recommended_quality: QualityPreset,
    pub recommended_target_ssim: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OptimizeResponse {
    pub optimized_svg: String,
    pub precision: u32,
    pub original_size: usize,
    pub optimized_size: usize,
    pub reduction_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BenchmarkRequest {
    pub target_ssim: f64,
    pub max_file_size: usize,
    pub max_iterations: usize,
    pub quality: QualityPreset,
}

impl Default for BenchmarkRequest {
    fn default() -> Self {
        Self {
            target_ssim: 0.998,
            max_file_size: 100_000,
            max_iterations: 4,
            quality: QualityPreset::Ultra,
        }
    }
}

pub fn vectorize_path(input_path: &Path, request: &VectorizeRequest) -> Result<VectorizeResponse> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_image_data(&image, request)
}

pub fn vectorize_bytes(input: &[u8], request: &VectorizeRequest) -> Result<VectorizeResponse> {
    let image = image::load_from_memory(input).context("unable to decode raster bytes")?;
    vectorize_image_data(&image, request)
}

pub fn analyze_path(input_path: &Path) -> Result<AnalyzeResponse> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    Ok(analyze_image_data(&image))
}

pub fn analyze_bytes(input: &[u8]) -> Result<AnalyzeResponse> {
    let image = image::load_from_memory(input).context("unable to decode raster bytes")?;
    Ok(analyze_image_data(&image))
}

pub fn inspect_path(input_path: &Path) -> Result<InfoResponse> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    let metadata = std::fs::metadata(input_path)?;
    Ok(build_info_response(
        &image,
        Some(metadata.len()),
        input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_string()),
    ))
}

pub fn compare_path(input_path: &Path, svg: &str) -> Result<QualityMetrics> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    compute_metrics(&image, svg)
}

pub fn compare_bytes(input: &[u8], svg: &str) -> Result<QualityMetrics> {
    let image = image::load_from_memory(input).context("unable to decode raster bytes")?;
    compute_metrics(&image, svg)
}

pub fn optimize(svg: &str, precision: u32) -> OptimizeResponse {
    let optimized_svg = optimize_svg(svg, precision);
    let original_size = svg.len();
    let optimized_size = optimized_svg.len();
    let reduction_percent = if original_size == 0 {
        0.0
    } else {
        (1.0 - optimized_size as f64 / original_size as f64) * 100.0
    };

    OptimizeResponse {
        optimized_svg,
        precision,
        original_size,
        optimized_size,
        reduction_percent,
    }
}

pub fn render_png(svg: &str, width: u32, height: u32) -> Result<Vec<u8>> {
    let rendered = render_svg_to_image(svg, width, height)?;
    let mut output = Cursor::new(Vec::new());
    let encoder = PngEncoder::new(&mut output);
    encoder.write_image(
        rendered.as_raw(),
        rendered.width(),
        rendered.height(),
        ColorType::Rgba8.into(),
    )?;
    Ok(output.into_inner())
}

pub fn benchmark(
    input_dir: &Path,
    output_dir: &Path,
    request: &BenchmarkRequest,
) -> Result<BenchmarkReport> {
    benchmark_directory(input_dir, output_dir, &request.to_vectorize_options())
}

pub fn benchmark_golden(
    golden_dir: &Path,
    work_dir: &Path,
    request: &BenchmarkRequest,
    limit: Option<usize>,
) -> Result<BenchmarkReport> {
    benchmark_golden_data(golden_dir, work_dir, &request.to_vectorize_options(), limit)
}

fn vectorize_image_data(
    original: &DynamicImage,
    request: &VectorizeRequest,
) -> Result<VectorizeResponse> {
    let default_logo_quality = logo_quality_from_quality(request.quality);
    let decision = matches!(request.method, VectorizeMethod::Auto | VectorizeMethod::Sam)
        .then(|| determine_auto_mode_image(original));

    let (svg, report, effective_method, fallback_from) = match request.method {
        VectorizeMethod::Hifi => {
            let (svg, report) = vectorize_image(original, &request.to_vectorize_options())?;
            (svg, report, VectorizeMethod::Hifi, None)
        }
        VectorizeMethod::Logo => {
            let (svg, report) = vectorize_logo_premium_image(
                original,
                request.logo_quality.unwrap_or(default_logo_quality),
                request.colors,
            )?;
            (svg, report, VectorizeMethod::Logo, None)
        }
        VectorizeMethod::Premium => {
            let (svg, report) =
                vectorize_premium_image(original, request.target_ssim, request.colors)?;
            (svg, report, VectorizeMethod::Premium, None)
        }
        VectorizeMethod::Auto => {
            let (svg, report) = vectorize_auto_image(original)?;
            (svg, report, VectorizeMethod::Auto, None)
        }
        VectorizeMethod::Smart => {
            let (svg, report) = vectorize_smart_image(
                original,
                request.target_ssim,
                request.max_file_size,
                request.max_iterations,
            )?;
            (svg, report, VectorizeMethod::Smart, None)
        }
        VectorizeMethod::Optimal => {
            let (svg, report) = vectorize_optimal_image(original)?;
            (svg, report, VectorizeMethod::Optimal, None)
        }
        VectorizeMethod::Bayesian => {
            let (svg, report) = vectorize_smart_image(
                original,
                request.target_ssim.max(0.95),
                request.max_file_size,
                request.max_iterations.max(5),
            )?;
            (svg, report, VectorizeMethod::Bayesian, None)
        }
        VectorizeMethod::Sam => {
            let (svg, report) = vectorize_auto_image(original)?;
            (
                svg,
                report,
                VectorizeMethod::Auto,
                Some(VectorizeMethod::Sam),
            )
        }
    };

    Ok(VectorizeResponse {
        svg,
        report,
        requested_method: request.method,
        effective_method,
        fallback_from,
        decision,
    })
}

fn analyze_image_data(image: &DynamicImage) -> AnalyzeResponse {
    AnalyzeResponse {
        analysis: crate::analysis::analyze_image(image),
        decision: determine_auto_mode_image(image),
    }
}

fn build_info_response(
    image: &DynamicImage,
    file_size_bytes: Option<u64>,
    format: Option<String>,
) -> InfoResponse {
    let analysis = crate::analysis::analyze_image(image);
    let color = image.color();
    let channels = color.channel_count();
    let color_mode = match channels {
        1 => "grayscale",
        3 => "rgb",
        4 => "rgba",
        _ => "unknown",
    }
    .to_string();

    let recommended_method = if analysis.width.max(analysis.height) <= 512 {
        VectorizeMethod::Hifi
    } else {
        VectorizeMethod::Premium
    };
    let recommended_quality = if analysis.width.max(analysis.height) <= 512 {
        QualityPreset::Ultra
    } else {
        QualityPreset::Balanced
    };
    let recommended_target_ssim = if analysis.width.max(analysis.height) <= 512 {
        0.998
    } else {
        0.995
    };

    InfoResponse {
        file_size_bytes,
        format,
        channels,
        color_mode,
        analysis,
        recommended_method,
        recommended_quality,
        recommended_target_ssim,
    }
}

fn logo_quality_from_quality(quality: QualityPreset) -> LogoQualityPreset {
    match quality {
        QualityPreset::Figma => LogoQualityPreset::Clean,
        QualityPreset::Balanced => LogoQualityPreset::Balanced,
        QualityPreset::Quality => LogoQualityPreset::High,
        QualityPreset::Ultra => LogoQualityPreset::Ultra,
    }
}

impl VectorizeRequest {
    fn to_vectorize_options(&self) -> VectorizeOptions {
        VectorizeOptions {
            target_ssim: self.target_ssim,
            max_file_size: self.max_file_size,
            max_iterations: self.max_iterations,
            quality: Some(self.quality),
        }
    }
}

impl BenchmarkRequest {
    fn to_vectorize_options(&self) -> VectorizeOptions {
        VectorizeOptions {
            target_ssim: self.target_ssim,
            max_file_size: self.max_file_size,
            max_iterations: self.max_iterations,
            quality: Some(self.quality),
        }
    }
}
