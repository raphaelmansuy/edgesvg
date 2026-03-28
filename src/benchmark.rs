use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use walkdir::WalkDir;

use crate::metrics::{
    render_svg_file_to_png_with_min_longest_side, BENCHMARK_REFERENCE_MIN_LONGEST_SIDE,
};
use crate::pipeline::{vectorize, write_svg, VectorizeOptions};
use crate::types::VectorizationReport;

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkEntry {
    pub input: String,
    pub output: String,
    pub reference: Option<String>,
    pub group: String,
    pub elapsed_ms: u64,
    pub report: VectorizationReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkGroupSummary {
    pub group: String,
    pub entries: usize,
    pub average_fidelity_score: f64,
    pub average_ssim: f64,
    pub average_psnr: f64,
    pub average_mae: f64,
    pub average_file_size: f64,
    pub average_path_count: f64,
    pub average_edge_similarity: f64,
    pub average_edge_f1: f64,
    pub average_foreground_iou: f64,
    pub average_color_similarity: f64,
    pub average_topology_score: f64,
    pub average_elapsed_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub entries: Vec<BenchmarkEntry>,
    pub groups: Vec<BenchmarkGroupSummary>,
    pub average_fidelity_score: f64,
    pub average_ssim: f64,
    pub average_psnr: f64,
    pub average_mae: f64,
    pub average_file_size: f64,
    pub average_path_count: f64,
    pub average_edge_similarity: f64,
    pub average_edge_f1: f64,
    pub average_foreground_iou: f64,
    pub average_color_similarity: f64,
    pub average_topology_score: f64,
    pub average_elapsed_ms: f64,
    pub total_elapsed_ms: u64,
    pub throughput_images_per_sec: f64,
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
        let started = Instant::now();
        let (svg, report) = vectorize(&input, options)?;
        write_svg(&output, &svg)?;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        entries.push(BenchmarkEntry {
            input: input.display().to_string(),
            output: output.display().to_string(),
            reference: None,
            group: benchmark_group(&input, input_dir),
            elapsed_ms,
            report,
        });
    }

    Ok(summarize_entries(entries))
}

pub fn benchmark_golden_data(
    golden_dir: &Path,
    work_dir: &Path,
    options: &VectorizeOptions,
    limit: Option<usize>,
) -> Result<BenchmarkReport> {
    let raster_dir = work_dir.join("rendered_inputs");
    let output_dir = work_dir.join("vectorized");
    if raster_dir.exists() {
        std::fs::remove_dir_all(&raster_dir)?;
    }
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir)?;
    }
    std::fs::create_dir_all(&raster_dir)?;
    std::fs::create_dir_all(&output_dir)?;

    let mut entries = Vec::new();
    let mut svg_inputs = golden_svg_inputs(golden_dir);
    if let Some(limit) = limit {
        svg_inputs.truncate(limit);
    }

    for reference_svg in svg_inputs {
        let relative = reference_svg
            .strip_prefix(golden_dir)
            .unwrap_or(&reference_svg);
        let png_name = relative.with_extension("png");
        let raster_path = raster_dir.join(&png_name);
        if let Some(parent) = raster_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        render_svg_file_to_png_with_min_longest_side(
            &reference_svg,
            &raster_path,
            BENCHMARK_REFERENCE_MIN_LONGEST_SIDE,
        )?;

        let output = output_dir.join(relative).with_extension("svg");
        let started = Instant::now();
        let (svg, report) = vectorize(&raster_path, options)?;
        write_svg(&output, &svg)?;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        entries.push(BenchmarkEntry {
            input: raster_path.display().to_string(),
            output: output.display().to_string(),
            reference: Some(relative.display().to_string()),
            group: benchmark_group(relative, golden_dir),
            elapsed_ms,
            report,
        });
    }

    Ok(summarize_entries(entries))
}

impl BenchmarkReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Benchmark Report\n\n");
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "## Overall\n\n- Entries: {}\n- Average Fidelity Score: {:.4}\n- Average SSIM: {:.4}\n- Average PSNR: {:.2}\n- Average MAE: {:.2}\n- Average Edge Similarity: {:.4}\n- Average Edge F1: {:.4}\n- Average Foreground IoU: {:.4}\n- Average Color Similarity: {:.4}\n- Average Topology Score: {:.4}\n- Average Size: {:.1} KB\n- Average Paths: {:.1}\n- Average Time: {:.1} ms\n- Throughput: {:.2} images/s\n\n",
                self.entries.len(),
                self.average_fidelity_score,
                self.average_ssim,
                self.average_psnr,
                self.average_mae,
                self.average_edge_similarity,
                self.average_edge_f1,
                self.average_foreground_iou,
                self.average_color_similarity,
                self.average_topology_score,
                self.average_file_size / 1024.0,
                self.average_path_count,
                self.average_elapsed_ms,
                self.throughput_images_per_sec
            ),
        );

        out.push_str("## By Group\n\n");
        out.push_str(
            "| Group | Entries | Fidelity | SSIM | Edge F1 | Color | Size (KB) | Paths | Time (ms) |\n",
        );
        out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
        for group in &self.groups {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.1} | {:.1} | {:.1} |\n",
                    group.group,
                    group.entries,
                    group.average_fidelity_score,
                    group.average_ssim,
                    group.average_edge_f1,
                    group.average_color_similarity,
                    group.average_file_size / 1024.0,
                    group.average_path_count,
                    group.average_elapsed_ms
                ),
            );
        }

        out.push_str("\n## Entries\n\n");
        out.push_str(
            "| Input | Fidelity | SSIM | Edge F1 | Color | Size (KB) | Paths | Preset |\n",
        );
        out.push_str("|---|---:|---:|---:|---:|---:|---:|---|\n");
        for entry in &self.entries {
            let metrics = &entry.report.metrics;
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.1} | {} | {:?} |\n",
                    entry_label(entry),
                    metrics.fidelity_score,
                    metrics.ssim,
                    metrics.edge_f1,
                    metrics.color_similarity,
                    metrics.file_size as f64 / 1024.0,
                    metrics.path_count,
                    entry.report.quality_preset
                ),
            );
        }

        let mut worst = self.entries.iter().collect::<Vec<_>>();
        worst.sort_by(|left, right| {
            left.report
                .metrics
                .fidelity_score
                .partial_cmp(&right.report.metrics.fidelity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.push_str("\n## Lowest Fidelity Entries\n\n");
        out.push_str(
            "| Input | Fidelity | SSIM | Edge F1 | Foreground IoU | Color | Size (KB) |\n",
        );
        out.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
        for entry in worst.into_iter().take(5) {
            let metrics = &entry.report.metrics;
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.1} |\n",
                    entry_label(entry),
                    metrics.fidelity_score,
                    metrics.ssim,
                    metrics.edge_f1,
                    metrics.foreground_iou,
                    metrics.color_similarity,
                    metrics.file_size as f64 / 1024.0,
                ),
            );
        }
        out
    }
}

fn entry_label(entry: &BenchmarkEntry) -> String {
    entry.reference.clone().unwrap_or_else(|| {
        Path::new(&entry.input)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&entry.input)
            .to_string()
    })
}

fn summarize_entries(entries: Vec<BenchmarkEntry>) -> BenchmarkReport {
    let len = entries.len().max(1) as f64;
    let average_fidelity_score = entries
        .iter()
        .map(|e| e.report.metrics.fidelity_score)
        .sum::<f64>()
        / len;
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
    let average_edge_similarity = entries
        .iter()
        .map(|e| e.report.metrics.edge_similarity)
        .sum::<f64>()
        / len;
    let average_edge_f1 = entries
        .iter()
        .map(|e| e.report.metrics.edge_f1)
        .sum::<f64>()
        / len;
    let average_foreground_iou = entries
        .iter()
        .map(|e| e.report.metrics.foreground_iou)
        .sum::<f64>()
        / len;
    let average_color_similarity = entries
        .iter()
        .map(|e| e.report.metrics.color_similarity)
        .sum::<f64>()
        / len;
    let average_topology_score = entries
        .iter()
        .map(|e| e.report.metrics.topology_score)
        .sum::<f64>()
        / len;
    let total_elapsed_ms = entries.iter().map(|e| e.elapsed_ms).sum::<u64>();
    let average_elapsed_ms = total_elapsed_ms as f64 / len;
    let throughput_images_per_sec = if total_elapsed_ms == 0 {
        entries.len() as f64
    } else {
        entries.len() as f64 / (total_elapsed_ms as f64 / 1_000.0)
    };
    let groups = summarize_groups(&entries);

    BenchmarkReport {
        entries,
        groups,
        average_fidelity_score,
        average_ssim,
        average_psnr,
        average_mae,
        average_file_size,
        average_path_count,
        average_edge_similarity,
        average_edge_f1,
        average_foreground_iou,
        average_color_similarity,
        average_topology_score,
        average_elapsed_ms,
        total_elapsed_ms,
        throughput_images_per_sec,
    }
}

fn summarize_groups(entries: &[BenchmarkEntry]) -> Vec<BenchmarkGroupSummary> {
    let mut groups = std::collections::BTreeMap::<String, Vec<&BenchmarkEntry>>::new();
    for entry in entries {
        groups.entry(entry.group.clone()).or_default().push(entry);
    }

    groups
        .into_iter()
        .map(|(group, entries)| {
            let len = entries.len().max(1) as f64;
            BenchmarkGroupSummary {
                group,
                entries: entries.len(),
                average_fidelity_score: entries
                    .iter()
                    .map(|e| e.report.metrics.fidelity_score)
                    .sum::<f64>()
                    / len,
                average_ssim: entries.iter().map(|e| e.report.metrics.ssim).sum::<f64>() / len,
                average_psnr: entries.iter().map(|e| e.report.metrics.psnr).sum::<f64>() / len,
                average_mae: entries.iter().map(|e| e.report.metrics.mae).sum::<f64>() / len,
                average_file_size: entries
                    .iter()
                    .map(|e| e.report.metrics.file_size as f64)
                    .sum::<f64>()
                    / len,
                average_path_count: entries
                    .iter()
                    .map(|e| e.report.metrics.path_count as f64)
                    .sum::<f64>()
                    / len,
                average_edge_similarity: entries
                    .iter()
                    .map(|e| e.report.metrics.edge_similarity)
                    .sum::<f64>()
                    / len,
                average_edge_f1: entries
                    .iter()
                    .map(|e| e.report.metrics.edge_f1)
                    .sum::<f64>()
                    / len,
                average_foreground_iou: entries
                    .iter()
                    .map(|e| e.report.metrics.foreground_iou)
                    .sum::<f64>()
                    / len,
                average_color_similarity: entries
                    .iter()
                    .map(|e| e.report.metrics.color_similarity)
                    .sum::<f64>()
                    / len,
                average_topology_score: entries
                    .iter()
                    .map(|e| e.report.metrics.topology_score)
                    .sum::<f64>()
                    / len,
                average_elapsed_ms: entries.iter().map(|e| e.elapsed_ms as f64).sum::<f64>() / len,
            }
        })
        .collect()
}

fn benchmark_group(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or("root")
        .to_string()
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

fn golden_svg_inputs(dir: &Path) -> Vec<PathBuf> {
    let mut inputs = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("svg"))
                .unwrap_or(false)
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    inputs.sort();
    inputs
}
