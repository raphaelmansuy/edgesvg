use std::path::Path;

use edgesvg::{
    analyze_path, benchmark, benchmark_golden, compare_path, inspect_path, optimize, render_png,
    vectorize_path, BenchmarkRequest, LogoQualityPreset, QualityPreset, VectorizeMethod,
    VectorizeRequest,
};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

fn parse_method(method: &str) -> PyResult<VectorizeMethod> {
    match method {
        "hifi" => Ok(VectorizeMethod::Hifi),
        "logo" => Ok(VectorizeMethod::Logo),
        "premium" => Ok(VectorizeMethod::Premium),
        "auto" => Ok(VectorizeMethod::Auto),
        "smart" => Ok(VectorizeMethod::Smart),
        "optimal" => Ok(VectorizeMethod::Optimal),
        "bayesian" => Ok(VectorizeMethod::Bayesian),
        "sam" => Ok(VectorizeMethod::Sam),
        other => Err(PyRuntimeError::new_err(format!(
            "unknown method: {other}. valid: hifi, logo, premium, auto, smart, optimal, bayesian, sam"
        ))),
    }
}

fn parse_quality(quality: &str) -> PyResult<QualityPreset> {
    match quality {
        "figma" => Ok(QualityPreset::Figma),
        "balanced" => Ok(QualityPreset::Balanced),
        "quality" => Ok(QualityPreset::Quality),
        "ultra" => Ok(QualityPreset::Ultra),
        other => Err(PyRuntimeError::new_err(format!(
            "unknown quality: {other}. valid: figma, balanced, quality, ultra"
        ))),
    }
}

fn parse_logo_quality(quality: Option<&str>) -> PyResult<Option<LogoQualityPreset>> {
    match quality {
        None => Ok(None),
        Some("clean") => Ok(Some(LogoQualityPreset::Clean)),
        Some("balanced") => Ok(Some(LogoQualityPreset::Balanced)),
        Some("high") => Ok(Some(LogoQualityPreset::High)),
        Some("ultra") => Ok(Some(LogoQualityPreset::Ultra)),
        Some(other) => Err(PyRuntimeError::new_err(format!(
            "unknown logo quality: {other}. valid: clean, balanced, high, ultra"
        ))),
    }
}

fn to_json<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(|err| PyRuntimeError::new_err(err.to_string()))
}

fn map_err(err: anyhow::Error) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

#[pyfunction]
#[pyo3(signature = (
    input_path,
    *,
    method = "hifi",
    target_ssim = 0.998,
    max_file_size = 100_000,
    max_iterations = 4,
    quality = "ultra",
    logo_quality = None,
    colors = None,
))]
#[allow(clippy::too_many_arguments)]
fn vectorize_json(
    input_path: &str,
    method: &str,
    target_ssim: f64,
    max_file_size: usize,
    max_iterations: usize,
    quality: &str,
    logo_quality: Option<&str>,
    colors: Option<usize>,
) -> PyResult<String> {
    let request = VectorizeRequest {
        method: parse_method(method)?,
        target_ssim,
        max_file_size,
        max_iterations,
        quality: parse_quality(quality)?,
        logo_quality: parse_logo_quality(logo_quality)?,
        colors,
    };
    let response = vectorize_path(Path::new(input_path), &request).map_err(map_err)?;
    to_json(&response)
}

#[pyfunction]
fn analyze_json(input_path: &str) -> PyResult<String> {
    to_json(&analyze_path(Path::new(input_path)).map_err(map_err)?)
}

#[pyfunction]
fn inspect_json(input_path: &str) -> PyResult<String> {
    to_json(&inspect_path(Path::new(input_path)).map_err(map_err)?)
}

#[pyfunction]
fn compare_json(input_path: &str, svg: &str) -> PyResult<String> {
    to_json(&compare_path(Path::new(input_path), svg).map_err(map_err)?)
}

#[pyfunction]
#[pyo3(signature = (svg, *, precision = 2))]
fn optimize_json(svg: &str, precision: u32) -> PyResult<String> {
    to_json(&optimize(svg, precision))
}

#[pyfunction]
fn render_png_bytes<'py>(
    py: Python<'py>,
    svg: &str,
    width: u32,
    height: u32,
) -> PyResult<Bound<'py, PyBytes>> {
    let png = render_png(svg, width, height).map_err(map_err)?;
    Ok(PyBytes::new(py, &png))
}

#[pyfunction]
#[pyo3(signature = (
    input_dir,
    output_dir,
    *,
    target_ssim = 0.998,
    max_file_size = 100_000,
    max_iterations = 4,
    quality = "ultra",
))]
fn benchmark_json(
    input_dir: &str,
    output_dir: &str,
    target_ssim: f64,
    max_file_size: usize,
    max_iterations: usize,
    quality: &str,
) -> PyResult<String> {
    let request = BenchmarkRequest {
        target_ssim,
        max_file_size,
        max_iterations,
        quality: parse_quality(quality)?,
    };
    let report =
        benchmark(Path::new(input_dir), Path::new(output_dir), &request).map_err(map_err)?;
    to_json(&report)
}

#[pyfunction]
#[pyo3(signature = (
    golden_dir,
    work_dir,
    *,
    target_ssim = 0.998,
    max_file_size = 100_000,
    max_iterations = 4,
    quality = "figma",
    limit = None,
))]
fn benchmark_golden_json(
    golden_dir: &str,
    work_dir: &str,
    target_ssim: f64,
    max_file_size: usize,
    max_iterations: usize,
    quality: &str,
    limit: Option<usize>,
) -> PyResult<String> {
    let request = BenchmarkRequest {
        target_ssim,
        max_file_size,
        max_iterations,
        quality: parse_quality(quality)?,
    };
    let report = benchmark_golden(Path::new(golden_dir), Path::new(work_dir), &request, limit)
        .map_err(map_err)?;
    to_json(&report)
}

#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[pymodule]
fn _edgesvg(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(vectorize_json, m)?)?;
    m.add_function(wrap_pyfunction!(analyze_json, m)?)?;
    m.add_function(wrap_pyfunction!(inspect_json, m)?)?;
    m.add_function(wrap_pyfunction!(compare_json, m)?)?;
    m.add_function(wrap_pyfunction!(optimize_json, m)?)?;
    m.add_function(wrap_pyfunction!(render_png_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark_json, m)?)?;
    m.add_function(wrap_pyfunction!(benchmark_golden_json, m)?)?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    Ok(())
}
