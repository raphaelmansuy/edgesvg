use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use walkdir::WalkDir;

use crate::pipeline::{vectorize, write_svg, VectorizeOptions};
use crate::types::VectorizationReport;

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkEntry {
    pub input: String,
    pub output: String,
    pub report: VectorizationReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub entries: Vec<BenchmarkEntry>,
    pub average_ssim: f64,
    pub average_psnr: f64,
    pub average_mae: f64,
    pub average_file_size: f64,
    pub average_path_count: f64,
}

pub fn benchmark_directory(
    input_dir: &Path,
    output_dir: &Path,
    options: &VectorizeOptions,
) -> Result<BenchmarkReport> {
    std::fs::create_dir_all(output_dir)?;

    let mut entries = Vec::new();
    for input in raster_inputs(input_dir) {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("output");
        let output = output_dir.join(format!("{stem}.svg"));
        let (svg, report) = vectorize(&input, options)?;
        write_svg(&output, &svg)?;
        entries.push(BenchmarkEntry {
            input: input.display().to_string(),
            output: output.display().to_string(),
            report,
        });
    }

    let len = entries.len().max(1) as f64;
    let average_ssim = entries.iter().map(|e| e.report.metrics.ssim).sum::<f64>() / len;
    let average_psnr = entries.iter().map(|e| e.report.metrics.psnr).sum::<f64>() / len;
    let average_mae = entries.iter().map(|e| e.report.metrics.mae).sum::<f64>() / len;
    let average_file_size = entries
        .iter()
        .map(|e| e.report.metrics.file_size as f64)
        .sum::<f64>()
        / len;
    let average_path_count = entries
        .iter()
        .map(|e| e.report.metrics.path_count as f64)
        .sum::<f64>()
        / len;

    Ok(BenchmarkReport {
        entries,
        average_ssim,
        average_psnr,
        average_mae,
        average_file_size,
        average_path_count,
    })
}

impl BenchmarkReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Benchmark Report\n\n");
        out.push_str("| Input | SSIM | PSNR | MAE | Size (KB) | Paths | Preset |\n");
        out.push_str("|---|---:|---:|---:|---:|---:|---|\n");
        for entry in &self.entries {
            let metrics = &entry.report.metrics;
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {:.4} | {:.2} | {:.2} | {:.1} | {} | {:?} |\n",
                    Path::new(&entry.input)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&entry.input),
                    metrics.ssim,
                    metrics.psnr,
                    metrics.mae,
                    metrics.file_size as f64 / 1024.0,
                    metrics.path_count,
                    entry.report.quality_preset
                ),
            );
        }

        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "\nAverage SSIM: {:.4}\n\nAverage PSNR: {:.2}\n\nAverage MAE: {:.2}\n\nAverage Size: {:.1} KB\n\nAverage Paths: {:.1}\n",
                self.average_ssim,
                self.average_psnr,
                self.average_mae,
                self.average_file_size / 1024.0,
                self.average_path_count
            ),
        );
        out
    }
}

fn raster_inputs(dir: &Path) -> Vec<PathBuf> {
    let mut inputs = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "bmp" | "webp"
                    )
                })
                .unwrap_or(false)
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    inputs.sort();
    inputs
}
