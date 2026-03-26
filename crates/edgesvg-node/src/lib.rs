#![deny(clippy::all)]

use std::path::Path;

use edgesvg::{
    analyze_path, benchmark, benchmark_golden, compare_path, inspect_path, optimize, render_png,
    vectorize_path, BenchmarkRequest, LogoQualityPreset, QualityPreset, VectorizeMethod,
    VectorizeRequest,
};
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

#[napi(object)]
pub struct VectorizeOptions {
    pub method: Option<String>,
    pub target_ssim: Option<f64>,
    pub max_file_size: Option<u32>,
    pub max_iterations: Option<u32>,
    pub quality: Option<String>,
    pub logo_quality: Option<String>,
    pub colors: Option<u32>,
}

#[napi(object)]
pub struct BenchmarkOptions {
    pub target_ssim: Option<f64>,
    pub max_file_size: Option<u32>,
    pub max_iterations: Option<u32>,
    pub quality: Option<String>,
    pub limit: Option<u32>,
}

fn parse_method(method: Option<&str>) -> napi::Result<VectorizeMethod> {
    match method.unwrap_or("hifi") {
        "hifi" => Ok(VectorizeMethod::Hifi),
        "logo" => Ok(VectorizeMethod::Logo),
        "premium" => Ok(VectorizeMethod::Premium),
        "auto" => Ok(VectorizeMethod::Auto),
        "smart" => Ok(VectorizeMethod::Smart),
        "optimal" => Ok(VectorizeMethod::Optimal),
        "bayesian" => Ok(VectorizeMethod::Bayesian),
        "sam" => Ok(VectorizeMethod::Sam),
        other => Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("unknown method: {other}"),
        )),
    }
}

fn parse_quality(quality: Option<&str>, default: QualityPreset) -> napi::Result<QualityPreset> {
    match quality.unwrap_or(match default {
        QualityPreset::Figma => "figma",
        QualityPreset::Balanced => "balanced",
        QualityPreset::Quality => "quality",
        QualityPreset::Ultra => "ultra",
    }) {
        "figma" => Ok(QualityPreset::Figma),
        "balanced" => Ok(QualityPreset::Balanced),
        "quality" => Ok(QualityPreset::Quality),
        "ultra" => Ok(QualityPreset::Ultra),
        other => Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("unknown quality: {other}"),
        )),
    }
}

fn parse_logo_quality(quality: Option<&str>) -> napi::Result<Option<LogoQualityPreset>> {
    match quality {
        None => Ok(None),
        Some("clean") => Ok(Some(LogoQualityPreset::Clean)),
        Some("balanced") => Ok(Some(LogoQualityPreset::Balanced)),
        Some("high") => Ok(Some(LogoQualityPreset::High)),
        Some("ultra") => Ok(Some(LogoQualityPreset::Ultra)),
        Some(other) => Err(napi::Error::new(
            napi::Status::InvalidArg,
            format!("unknown logo quality: {other}"),
        )),
    }
}

fn as_json<T: serde::Serialize>(value: &T) -> napi::Result<String> {
    serde_json::to_string(value)
        .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))
}

#[napi]
pub fn vectorize_json(
    input_path: String,
    options: Option<VectorizeOptions>,
) -> napi::Result<String> {
    let options = options.unwrap_or(VectorizeOptions {
        method: None,
        target_ssim: None,
        max_file_size: None,
        max_iterations: None,
        quality: None,
        logo_quality: None,
        colors: None,
    });
    let request = VectorizeRequest {
        method: parse_method(options.method.as_deref())?,
        target_ssim: options.target_ssim.unwrap_or(0.998),
        max_file_size: options.max_file_size.unwrap_or(100_000) as usize,
        max_iterations: options.max_iterations.unwrap_or(4) as usize,
        quality: parse_quality(options.quality.as_deref(), QualityPreset::Ultra)?,
        logo_quality: parse_logo_quality(options.logo_quality.as_deref())?,
        colors: options.colors.map(|value| value as usize),
    };
    let response = vectorize_path(Path::new(&input_path), &request)
        .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?;
    as_json(&response)
}

#[napi]
pub fn analyze_json(input_path: String) -> napi::Result<String> {
    as_json(
        &analyze_path(Path::new(&input_path))
            .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?,
    )
}

#[napi]
pub fn inspect_json(input_path: String) -> napi::Result<String> {
    as_json(
        &inspect_path(Path::new(&input_path))
            .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?,
    )
}

#[napi]
pub fn compare_json(input_path: String, svg: String) -> napi::Result<String> {
    as_json(
        &compare_path(Path::new(&input_path), &svg)
            .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?,
    )
}

#[napi]
pub fn optimize_json(svg: String, precision: Option<u32>) -> napi::Result<String> {
    as_json(&optimize(&svg, precision.unwrap_or(2)))
}

#[napi]
pub fn render_png_buffer(svg: String, width: u32, height: u32) -> napi::Result<Buffer> {
    let png = render_png(&svg, width, height)
        .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?;
    Ok(Buffer::from(png))
}

#[napi]
pub fn benchmark_json(
    input_dir: String,
    output_dir: String,
    options: Option<BenchmarkOptions>,
) -> napi::Result<String> {
    let options = options.unwrap_or(BenchmarkOptions {
        target_ssim: None,
        max_file_size: None,
        max_iterations: None,
        quality: None,
        limit: None,
    });
    let request = BenchmarkRequest {
        target_ssim: options.target_ssim.unwrap_or(0.998),
        max_file_size: options.max_file_size.unwrap_or(100_000) as usize,
        max_iterations: options.max_iterations.unwrap_or(4) as usize,
        quality: parse_quality(options.quality.as_deref(), QualityPreset::Ultra)?,
    };
    let report = benchmark(Path::new(&input_dir), Path::new(&output_dir), &request)
        .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?;
    as_json(&report)
}

#[napi]
pub fn benchmark_golden_json(
    golden_dir: String,
    work_dir: String,
    options: Option<BenchmarkOptions>,
) -> napi::Result<String> {
    let options = options.unwrap_or(BenchmarkOptions {
        target_ssim: None,
        max_file_size: None,
        max_iterations: None,
        quality: None,
        limit: None,
    });
    let request = BenchmarkRequest {
        target_ssim: options.target_ssim.unwrap_or(0.998),
        max_file_size: options.max_file_size.unwrap_or(100_000) as usize,
        max_iterations: options.max_iterations.unwrap_or(4) as usize,
        quality: parse_quality(options.quality.as_deref(), QualityPreset::Figma)?,
    };
    let report = benchmark_golden(
        Path::new(&golden_dir),
        Path::new(&work_dir),
        &request,
        options.limit.map(|value| value as usize),
    )
    .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?;
    as_json(&report)
}

#[napi]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
