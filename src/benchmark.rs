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
    pub min_fidelity_score: f64,
    pub p10_fidelity_score: f64,
    pub average_ssim: f64,
    pub average_gradient_similarity: f64,
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
pub struct BenchmarkMetricDistribution {
    pub min: f64,
    pub p10: f64,
    pub median: f64,
    pub p90: f64,
    pub max: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkDistributionSummary {
    pub fidelity_score: BenchmarkMetricDistribution,
    pub ssim: BenchmarkMetricDistribution,
    pub edge_f1: BenchmarkMetricDistribution,
    pub color_similarity: BenchmarkMetricDistribution,
    pub topology_score: BenchmarkMetricDistribution,
    pub file_size: BenchmarkMetricDistribution,
    pub path_count: BenchmarkMetricDistribution,
    pub elapsed_ms: BenchmarkMetricDistribution,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkQualityGates {
    pub fidelity_below_0_75: usize,
    pub fidelity_below_0_85: usize,
    pub edge_f1_below_0_75: usize,
    pub edge_f1_below_0_85: usize,
    pub color_below_0_75: usize,
    pub topology_below_0_50: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkDatasetCount {
    pub label: String,
    pub entries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkDatasetSummary {
    pub groups: Vec<BenchmarkDatasetCount>,
    pub image_kinds: Vec<BenchmarkDatasetCount>,
    pub complexities: Vec<BenchmarkDatasetCount>,
    pub kind_complexities: Vec<BenchmarkDatasetCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub entries: Vec<BenchmarkEntry>,
    pub groups: Vec<BenchmarkGroupSummary>,
    pub dataset: BenchmarkDatasetSummary,
    pub distributions: BenchmarkDistributionSummary,
    pub quality_gates: BenchmarkQualityGates,
    pub robust_benchmark_score: f64,
    pub average_fidelity_score: f64,
    pub average_ssim: f64,
    pub average_gradient_similarity: f64,
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
    let artifacts_dir = work_dir.join("artifacts");
    let reports_dir = work_dir.join("reports");
    let raster_dir = artifacts_dir.join("rendered_inputs");
    let output_dir = artifacts_dir.join("vectorized");
    if raster_dir.exists() {
        std::fs::remove_dir_all(&raster_dir)?;
    }
    if output_dir.exists() {
        std::fs::remove_dir_all(&output_dir)?;
    }
    std::fs::create_dir_all(&raster_dir)?;
    std::fs::create_dir_all(&output_dir)?;
    std::fs::create_dir_all(&reports_dir)?;

    let mut entries = Vec::new();
    let svg_inputs = select_golden_svg_inputs(golden_dir, limit);
    write_dataset_manifest(golden_dir, &reports_dir, &svg_inputs)?;

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
                "## Overall\n\n- Entries: {}\n- Robust Benchmark Score: {:.4}\n- Average Fidelity Score: {:.4}\n- Fidelity p10 / min: {:.4} / {:.4}\n- Average SSIM: {:.4}\n- Average Gradient Similarity: {:.4}\n- Average PSNR: {:.2}\n- Average MAE: {:.2}\n- Average Edge Similarity: {:.4}\n- Average Edge F1: {:.4}\n- Edge F1 p10 / min: {:.4} / {:.4}\n- Average Foreground IoU: {:.4}\n- Average Color Similarity: {:.4}\n- Average Topology Score: {:.4}\n- Average Size: {:.1} KB\n- Size p90 / max: {:.1} KB / {:.1} KB\n- Average Paths: {:.1}\n- Paths p90 / max: {:.1} / {:.1}\n- Average Time: {:.1} ms\n- Throughput: {:.2} images/s\n\n",
                self.entries.len(),
                self.robust_benchmark_score,
                self.average_fidelity_score,
                self.distributions.fidelity_score.p10,
                self.distributions.fidelity_score.min,
                self.average_ssim,
                self.average_gradient_similarity,
                self.average_psnr,
                self.average_mae,
                self.average_edge_similarity,
                self.average_edge_f1,
                self.distributions.edge_f1.p10,
                self.distributions.edge_f1.min,
                self.average_foreground_iou,
                self.average_color_similarity,
                self.average_topology_score,
                self.average_file_size / 1024.0,
                self.distributions.file_size.p90 / 1024.0,
                self.distributions.file_size.max / 1024.0,
                self.average_path_count,
                self.distributions.path_count.p90,
                self.distributions.path_count.max,
                self.average_elapsed_ms,
                self.throughput_images_per_sec
            ),
        );

        out.push_str("## Dataset Mix\n\n");
        for item in &self.dataset.groups {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("- Group `{}`: {}\n", item.label, item.entries),
            );
        }
        for item in &self.dataset.kind_complexities {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("- Type `{}`: {}\n", item.label, item.entries),
            );
        }
        out.push('\n');

        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "## Quality Gates\n\n- Fidelity < 0.75: {}\n- Fidelity < 0.85: {}\n- Edge F1 < 0.75: {}\n- Edge F1 < 0.85: {}\n- Color < 0.75: {}\n- Topology < 0.50: {}\n\n",
                self.quality_gates.fidelity_below_0_75,
                self.quality_gates.fidelity_below_0_85,
                self.quality_gates.edge_f1_below_0_75,
                self.quality_gates.edge_f1_below_0_85,
                self.quality_gates.color_below_0_75,
                self.quality_gates.topology_below_0_50,
            ),
        );

        out.push_str("## By Group\n\n");
        out.push_str(
            "| Group | Entries | Fidelity | P10 Fidelity | Min Fidelity | SSIM | Gradient | Edge F1 | Color | Size (KB) | Paths | Time (ms) |\n",
        );
        out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
        for group in &self.groups {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.1} | {:.1} | {:.1} |\n",
                    group.group,
                    group.entries,
                    group.average_fidelity_score,
                    group.p10_fidelity_score,
                    group.min_fidelity_score,
                    group.average_ssim,
                    group.average_gradient_similarity,
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
            "| Input | Fidelity | SSIM | Gradient | Edge F1 | Color | Size (KB) | Paths | Preset |\n",
        );
        out.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---|\n");
        for entry in &self.entries {
            let metrics = &entry.report.metrics;
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.1} | {} | {:?} |\n",
                    entry_label(entry),
                    metrics.fidelity_score,
                    metrics.ssim,
                    metrics.gradient_similarity,
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
    let distributions = summarize_distributions(&entries);
    let dataset = summarize_dataset(&entries);
    let quality_gates = summarize_quality_gates(&entries);
    let average_fidelity_score = entries
        .iter()
        .map(|e| e.report.metrics.fidelity_score)
        .sum::<f64>()
        / len;
    let average_ssim = entries.iter().map(|e| e.report.metrics.ssim).sum::<f64>() / len;
    let average_gradient_similarity = entries
        .iter()
        .map(|e| e.report.metrics.gradient_similarity)
        .sum::<f64>()
        / len;
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
    let robust_benchmark_score = robust_benchmark_score(&distributions, &quality_gates);

    BenchmarkReport {
        entries,
        groups,
        dataset,
        distributions,
        quality_gates,
        robust_benchmark_score,
        average_fidelity_score,
        average_ssim,
        average_gradient_similarity,
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
            let fidelity = distribution(entries.iter().map(|e| e.report.metrics.fidelity_score));
            BenchmarkGroupSummary {
                group,
                entries: entries.len(),
                average_fidelity_score: entries
                    .iter()
                    .map(|e| e.report.metrics.fidelity_score)
                    .sum::<f64>()
                    / len,
                min_fidelity_score: fidelity.min,
                p10_fidelity_score: fidelity.p10,
                average_ssim: entries.iter().map(|e| e.report.metrics.ssim).sum::<f64>() / len,
                average_gradient_similarity: entries
                    .iter()
                    .map(|e| e.report.metrics.gradient_similarity)
                    .sum::<f64>()
                    / len,
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

fn summarize_distributions(entries: &[BenchmarkEntry]) -> BenchmarkDistributionSummary {
    BenchmarkDistributionSummary {
        fidelity_score: distribution(entries.iter().map(|e| e.report.metrics.fidelity_score)),
        ssim: distribution(entries.iter().map(|e| e.report.metrics.ssim)),
        edge_f1: distribution(entries.iter().map(|e| e.report.metrics.edge_f1)),
        color_similarity: distribution(entries.iter().map(|e| e.report.metrics.color_similarity)),
        topology_score: distribution(entries.iter().map(|e| e.report.metrics.topology_score)),
        file_size: distribution(entries.iter().map(|e| e.report.metrics.file_size as f64)),
        path_count: distribution(entries.iter().map(|e| e.report.metrics.path_count as f64)),
        elapsed_ms: distribution(entries.iter().map(|e| e.elapsed_ms as f64)),
    }
}

fn distribution(values: impl Iterator<Item = f64>) -> BenchmarkMetricDistribution {
    let mut values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return BenchmarkMetricDistribution {
            min: 0.0,
            p10: 0.0,
            median: 0.0,
            p90: 0.0,
            max: 0.0,
        };
    }

    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    BenchmarkMetricDistribution {
        min: *values.first().unwrap_or(&0.0),
        p10: percentile(&values, 0.10),
        median: percentile(&values, 0.50),
        p90: percentile(&values, 0.90),
        max: *values.last().unwrap_or(&0.0),
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

fn summarize_quality_gates(entries: &[BenchmarkEntry]) -> BenchmarkQualityGates {
    BenchmarkQualityGates {
        fidelity_below_0_75: entries
            .iter()
            .filter(|entry| entry.report.metrics.fidelity_score < 0.75)
            .count(),
        fidelity_below_0_85: entries
            .iter()
            .filter(|entry| entry.report.metrics.fidelity_score < 0.85)
            .count(),
        edge_f1_below_0_75: entries
            .iter()
            .filter(|entry| entry.report.metrics.edge_f1 < 0.75)
            .count(),
        edge_f1_below_0_85: entries
            .iter()
            .filter(|entry| entry.report.metrics.edge_f1 < 0.85)
            .count(),
        color_below_0_75: entries
            .iter()
            .filter(|entry| entry.report.metrics.color_similarity < 0.75)
            .count(),
        topology_below_0_50: entries
            .iter()
            .filter(|entry| entry.report.metrics.topology_score < 0.50)
            .count(),
    }
}

fn summarize_dataset(entries: &[BenchmarkEntry]) -> BenchmarkDatasetSummary {
    BenchmarkDatasetSummary {
        groups: count_labels(entries.iter().map(|entry| entry.group.clone())),
        image_kinds: count_labels(entries.iter().map(|entry| {
            format!("{:?}", entry.report.analysis.image_type).to_lowercase()
        })),
        complexities: count_labels(entries.iter().map(|entry| {
            format!("{:?}", entry.report.analysis.complexity).to_lowercase()
        })),
        kind_complexities: count_labels(entries.iter().map(|entry| {
            format!(
                "{:?}/{:?}",
                entry.report.analysis.image_type, entry.report.analysis.complexity
            )
            .to_lowercase()
        })),
    }
}

fn count_labels(labels: impl Iterator<Item = String>) -> Vec<BenchmarkDatasetCount> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for label in labels {
        *counts.entry(label).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(label, entries)| BenchmarkDatasetCount { label, entries })
        .collect()
}

fn robust_benchmark_score(
    distributions: &BenchmarkDistributionSummary,
    quality_gates: &BenchmarkQualityGates,
) -> f64 {
    let quality_core = distributions.fidelity_score.median * 0.20
        + distributions.fidelity_score.p10 * 0.25
        + distributions.fidelity_score.min * 0.15
        + distributions.edge_f1.median * 0.10
        + distributions.edge_f1.p10 * 0.12
        + distributions.color_similarity.p10 * 0.08
        + distributions.topology_score.p10 * 0.10;
    let compactness = (1.0 - (distributions.file_size.p90 / 20_000.0)).clamp(0.0, 1.0) * 0.06
        + (1.0 - (distributions.path_count.p90 / 120.0)).clamp(0.0, 1.0) * 0.04;
    let gate_penalty = quality_gates.fidelity_below_0_75 as f64 * 0.025
        + quality_gates.edge_f1_below_0_75 as f64 * 0.015
        + quality_gates.topology_below_0_50 as f64 * 0.01;

    (quality_core + compactness - gate_penalty).clamp(0.0, 1.0)
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

fn select_golden_svg_inputs(dir: &Path, limit: Option<usize>) -> Vec<PathBuf> {
    let inputs = golden_svg_inputs(dir);
    let Some(limit) = limit else {
        return inputs;
    };
    if limit >= inputs.len() {
        return inputs;
    }

    let mut by_group = std::collections::BTreeMap::<String, Vec<PathBuf>>::new();
    for input in inputs {
        by_group
            .entry(benchmark_group(&input, dir))
            .or_default()
            .push(input);
    }

    let groups = by_group.keys().cloned().collect::<Vec<_>>();
    if groups.is_empty() {
        return Vec::new();
    }

    let mut allocations = std::collections::BTreeMap::<String, usize>::new();
    let mut remaining = limit;

    if limit < groups.len() {
        for group in groups.iter().take(limit) {
            allocations.insert(group.clone(), 1);
        }
    } else {
        for group in &groups {
            let has_capacity = by_group.get(group).map(|items| !items.is_empty()).unwrap_or(false);
            let allocation = usize::from(has_capacity);
            allocations.insert(group.clone(), allocation);
            remaining = remaining.saturating_sub(allocation);
        }

        while remaining > 0 {
            let mut progressed = false;
            for group in &groups {
                let current = *allocations.get(group).unwrap_or(&0);
                let capacity = by_group.get(group).map(|items| items.len()).unwrap_or(0);
                if current < capacity {
                    allocations.insert(group.clone(), current + 1);
                    remaining -= 1;
                    progressed = true;
                    if remaining == 0 {
                        break;
                    }
                }
            }
            if !progressed {
                break;
            }
        }
    }

    let mut selected = Vec::new();
    for group in groups {
        let Some(items) = by_group.get(&group) else {
            continue;
        };
        let allocation = *allocations.get(&group).unwrap_or(&0);
        selected.extend(sample_evenly(items, allocation));
    }
    selected.sort();
    selected
}

fn sample_evenly(items: &[PathBuf], count: usize) -> Vec<PathBuf> {
    if count == 0 {
        return Vec::new();
    }
    if count >= items.len() {
        return items.to_vec();
    }

    let total = items.len();
    (0..count)
        .map(|index| {
            let centered = (2 * index + 1) * total;
            let position = centered / (2 * count);
            items[position.min(total - 1)].clone()
        })
        .collect()
}

fn write_dataset_manifest(golden_dir: &Path, reports_dir: &Path, inputs: &[PathBuf]) -> Result<()> {
    let manifest = inputs
        .iter()
        .map(|input| {
            let relative = input.strip_prefix(golden_dir).unwrap_or(input);
            serde_json::json!({
                "reference": relative.display().to_string(),
                "group": benchmark_group(relative, golden_dir),
            })
        })
        .collect::<Vec<_>>();
    std::fs::write(
        reports_dir.join("dataset_manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::select_golden_svg_inputs;

    #[test]
    fn limited_selection_balances_across_top_level_groups() {
        let dir = tempdir().unwrap();
        for group in ["icons", "logos", "illustrations"] {
            let group_dir = dir.path().join(group);
            std::fs::create_dir_all(&group_dir).unwrap();
            for index in 0..4 {
                std::fs::write(
                    group_dir.join(format!("{group}_{index}.svg")),
                    r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="#000"/></svg>"##,
                )
                .unwrap();
            }
        }

        let selected = select_golden_svg_inputs(dir.path(), Some(6));
        let labels = selected
            .iter()
            .map(|path| {
                path.strip_prefix(dir.path())
                    .unwrap()
                    .components()
                    .next()
                    .unwrap()
                    .as_os_str()
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(selected.len(), 6);
        assert_eq!(labels.iter().filter(|label| *label == "icons").count(), 2);
        assert_eq!(labels.iter().filter(|label| *label == "logos").count(), 2);
        assert_eq!(labels.iter().filter(|label| *label == "illustrations").count(), 2);
    }
}
