use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use walkdir::WalkDir;

use crate::metrics::{
    render_svg_file_to_png_with_min_longest_side, BENCHMARK_REFERENCE_MIN_LONGEST_SIDE,
};
use crate::pipeline::{vectorize, write_svg, VectorizeOptions};
use crate::types::{Complexity, ImageAnalysis, ImageKind, VectorizationReport};

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkEntry {
    pub input: String,
    pub output: String,
    pub reference: Option<String>,
    pub reference_subgroup: Option<String>,
    pub reference_complexity: Option<String>,
    pub group: String,
    pub elapsed_ms: u64,
    pub report: VectorizationReport,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkGroupSummary {
    pub group: String,
    pub entries: usize,
    pub average_reliability_score: f64,
    pub average_efficiency_score: f64,
    pub min_reliability_score: f64,
    pub p10_reliability_score: f64,
    pub average_fidelity_score: f64,
    pub min_fidelity_score: f64,
    pub p10_fidelity_score: f64,
    pub average_ssim: f64,
    pub average_local_ssim_p10: f64,
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
    pub reliability_score: BenchmarkMetricDistribution,
    pub efficiency_score: BenchmarkMetricDistribution,
    pub fidelity_score: BenchmarkMetricDistribution,
    pub ssim: BenchmarkMetricDistribution,
    pub local_ssim_p10: BenchmarkMetricDistribution,
    pub edge_f1: BenchmarkMetricDistribution,
    pub color_similarity: BenchmarkMetricDistribution,
    pub topology_score: BenchmarkMetricDistribution,
    pub file_size: BenchmarkMetricDistribution,
    pub path_count: BenchmarkMetricDistribution,
    pub elapsed_ms: BenchmarkMetricDistribution,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkQualityGates {
    pub reliability_below_0_80: usize,
    pub fidelity_below_0_75: usize,
    pub fidelity_below_0_85: usize,
    pub local_ssim_p10_below_0_75: usize,
    pub edge_f1_below_0_75: usize,
    pub edge_f1_below_0_85: usize,
    pub color_below_0_75: usize,
    pub topology_below_0_50: usize,
    pub efficiency_below_0_50: usize,
    pub size_budget_failures: usize,
    pub path_budget_failures: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkDatasetCount {
    pub label: String,
    pub entries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkDatasetSummary {
    pub groups: Vec<BenchmarkDatasetCount>,
    pub subgroups: Vec<BenchmarkDatasetCount>,
    pub image_kinds: Vec<BenchmarkDatasetCount>,
    pub complexities: Vec<BenchmarkDatasetCount>,
    pub kind_complexities: Vec<BenchmarkDatasetCount>,
    pub reference_complexities: Vec<BenchmarkDatasetCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub entries: Vec<BenchmarkEntry>,
    pub groups: Vec<BenchmarkGroupSummary>,
    pub dataset: BenchmarkDatasetSummary,
    pub distributions: BenchmarkDistributionSummary,
    pub quality_gates: BenchmarkQualityGates,
    pub robust_benchmark_score: f64,
    pub balanced_group_score: f64,
    pub balanced_subgroup_score: f64,
    pub balanced_kind_complexity_score: f64,
    pub dataset_health_score: f64,
    pub average_reliability_score: f64,
    pub average_efficiency_score: f64,
    pub average_fidelity_score: f64,
    pub average_ssim: f64,
    pub average_local_ssim_p10: f64,
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
            reference_subgroup: None,
            reference_complexity: None,
            group: benchmark_group(&input, input_dir),
            elapsed_ms,
            report,
        });
    }

    let report = summarize_entries(entries);
    write_structured_reports(&output_dir.join("_benchmark_reports"), &report, None)?;
    Ok(report)
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

    for reference_input in svg_inputs {
        let reference_svg = reference_input.path;
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
            reference_subgroup: Some(reference_input.subgroup),
            reference_complexity: Some(reference_input.reference_complexity),
            group: reference_input.group,
            elapsed_ms,
            report,
        });
    }

    let report = summarize_entries(entries);
    write_structured_reports(
        &reports_dir,
        &report,
        Some(reports_dir.join("dataset_manifest.json")),
    )?;
    Ok(report)
}

impl BenchmarkReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::from("# Benchmark Report\n\n");
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "## Overall\n\n- Entries: {}\n- Robust Benchmark Score: {:.4}\n- Dataset Health Score: {:.4}\n- Balanced Group Score: {:.4}\n- Balanced Subgroup Score: {:.4}\n- Balanced Type/Complexity Score: {:.4}\n- Average Reliability Score: {:.4}\n- Average Efficiency Score: {:.4}\n- Reliability p10 / min: {:.4} / {:.4}\n- Average Fidelity Score: {:.4}\n- Fidelity p10 / min: {:.4} / {:.4}\n- Average SSIM: {:.4}\n- Average Local SSIM p10: {:.4}\n- Local SSIM p10 p10 / min: {:.4} / {:.4}\n- Average Gradient Similarity: {:.4}\n- Average PSNR: {:.2}\n- Average MAE: {:.2}\n- Average Edge Similarity: {:.4}\n- Average Edge F1: {:.4}\n- Edge F1 p10 / min: {:.4} / {:.4}\n- Average Foreground IoU: {:.4}\n- Average Color Similarity: {:.4}\n- Average Topology Score: {:.4}\n- Average Size: {:.1} KB\n- Size p90 / max: {:.1} KB / {:.1} KB\n- Average Paths: {:.1}\n- Paths p90 / max: {:.1} / {:.1}\n- Average Time: {:.1} ms\n- Throughput: {:.2} images/s\n\n",
                self.entries.len(),
                self.robust_benchmark_score,
                self.dataset_health_score,
                self.balanced_group_score,
                self.balanced_subgroup_score,
                self.balanced_kind_complexity_score,
                self.average_reliability_score,
                self.average_efficiency_score,
                self.distributions.reliability_score.p10,
                self.distributions.reliability_score.min,
                self.average_fidelity_score,
                self.distributions.fidelity_score.p10,
                self.distributions.fidelity_score.min,
                self.average_ssim,
                self.average_local_ssim_p10,
                self.distributions.local_ssim_p10.p10,
                self.distributions.local_ssim_p10.min,
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
        for item in &self.dataset.reference_complexities {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("- Source complexity `{}`: {}\n", item.label, item.entries),
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
                "## Quality Gates\n\n- Reliability < 0.80: {}\n- Fidelity < 0.75: {}\n- Fidelity < 0.85: {}\n- Local SSIM p10 < 0.75: {}\n- Edge F1 < 0.75: {}\n- Edge F1 < 0.85: {}\n- Color < 0.75: {}\n- Topology < 0.50: {}\n- Efficiency < 0.50: {}\n- Size budget failures: {}\n- Path budget failures: {}\n\n",
                self.quality_gates.reliability_below_0_80,
                self.quality_gates.fidelity_below_0_75,
                self.quality_gates.fidelity_below_0_85,
                self.quality_gates.local_ssim_p10_below_0_75,
                self.quality_gates.edge_f1_below_0_75,
                self.quality_gates.edge_f1_below_0_85,
                self.quality_gates.color_below_0_75,
                self.quality_gates.topology_below_0_50,
                self.quality_gates.efficiency_below_0_50,
                self.quality_gates.size_budget_failures,
                self.quality_gates.path_budget_failures,
            ),
        );

        out.push_str("## By Group\n\n");
        out.push_str(
            "| Group | Entries | Reliability | Efficiency | P10 Rel | Min Rel | Fidelity | P10 Fidelity | Min Fidelity | SSIM | Local SSIM p10 | Gradient | Edge F1 | Color | Size (KB) | Paths | Time (ms) |\n",
        );
        out.push_str(
            "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
        );
        for group in &self.groups {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!(
                    "| {} | {} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.1} | {:.1} | {:.1} |\n",
                    group.group,
                    group.entries,
                    group.average_reliability_score,
                    group.average_efficiency_score,
                    group.p10_reliability_score,
                    group.min_reliability_score,
                    group.average_fidelity_score,
                    group.p10_fidelity_score,
                    group.min_fidelity_score,
                    group.average_ssim,
                    group.average_local_ssim_p10,
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
    let average_reliability_score = entries
        .iter()
        .map(|e| reliability_score(&e.report.metrics))
        .sum::<f64>()
        / len;
    let average_efficiency_score =
        entries.iter().map(benchmark_efficiency_score).sum::<f64>() / len;
    let average_fidelity_score = entries
        .iter()
        .map(|e| e.report.metrics.fidelity_score)
        .sum::<f64>()
        / len;
    let average_ssim = entries.iter().map(|e| e.report.metrics.ssim).sum::<f64>() / len;
    let average_local_ssim_p10 = entries
        .iter()
        .map(|e| e.report.metrics.local_ssim_p10)
        .sum::<f64>()
        / len;
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
    let balanced_group_score = balanced_score_by_label(&entries, |entry| entry.group.clone());
    let balanced_subgroup_score = balanced_score_by_label(&entries, |entry| {
        entry
            .reference_subgroup
            .clone()
            .unwrap_or_else(|| "root".to_string())
    });
    let balanced_kind_complexity_score = balanced_score_by_label(&entries, |entry| {
        format!(
            "{:?}/{:?}",
            entry.report.analysis.image_type, entry.report.analysis.complexity
        )
        .to_lowercase()
    });
    let dataset_health_score = dataset_health_score(&dataset);
    let robust_benchmark_score = robust_benchmark_score(
        &distributions,
        &quality_gates,
        dataset_health_score,
        balanced_group_score,
        balanced_subgroup_score,
        balanced_kind_complexity_score,
    );

    BenchmarkReport {
        entries,
        groups,
        dataset,
        distributions,
        quality_gates,
        robust_benchmark_score,
        balanced_group_score,
        balanced_subgroup_score,
        balanced_kind_complexity_score,
        dataset_health_score,
        average_reliability_score,
        average_efficiency_score,
        average_fidelity_score,
        average_ssim,
        average_local_ssim_p10,
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
            let reliability =
                distribution(entries.iter().map(|e| reliability_score(&e.report.metrics)));
            let fidelity = distribution(entries.iter().map(|e| e.report.metrics.fidelity_score));
            BenchmarkGroupSummary {
                group,
                entries: entries.len(),
                average_reliability_score: entries
                    .iter()
                    .map(|e| reliability_score(&e.report.metrics))
                    .sum::<f64>()
                    / len,
                average_efficiency_score: entries
                    .iter()
                    .map(|entry| benchmark_efficiency_score(entry))
                    .sum::<f64>()
                    / len,
                min_reliability_score: reliability.min,
                p10_reliability_score: reliability.p10,
                average_fidelity_score: entries
                    .iter()
                    .map(|e| e.report.metrics.fidelity_score)
                    .sum::<f64>()
                    / len,
                min_fidelity_score: fidelity.min,
                p10_fidelity_score: fidelity.p10,
                average_ssim: entries.iter().map(|e| e.report.metrics.ssim).sum::<f64>() / len,
                average_local_ssim_p10: entries
                    .iter()
                    .map(|e| e.report.metrics.local_ssim_p10)
                    .sum::<f64>()
                    / len,
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
        reliability_score: distribution(
            entries.iter().map(|e| reliability_score(&e.report.metrics)),
        ),
        efficiency_score: distribution(entries.iter().map(benchmark_efficiency_score)),
        fidelity_score: distribution(entries.iter().map(|e| e.report.metrics.fidelity_score)),
        ssim: distribution(entries.iter().map(|e| e.report.metrics.ssim)),
        local_ssim_p10: distribution(entries.iter().map(|e| e.report.metrics.local_ssim_p10)),
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
        reliability_below_0_80: entries
            .iter()
            .filter(|entry| reliability_score(&entry.report.metrics) < 0.80)
            .count(),
        fidelity_below_0_75: entries
            .iter()
            .filter(|entry| entry.report.metrics.fidelity_score < 0.75)
            .count(),
        fidelity_below_0_85: entries
            .iter()
            .filter(|entry| entry.report.metrics.fidelity_score < 0.85)
            .count(),
        local_ssim_p10_below_0_75: entries
            .iter()
            .filter(|entry| entry.report.metrics.local_ssim_p10 < 0.75)
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
        efficiency_below_0_50: entries
            .iter()
            .filter(|entry| benchmark_efficiency_score(entry) < 0.50)
            .count(),
        size_budget_failures: entries
            .iter()
            .filter(|entry| {
                let (size_budget, _) = benchmark_complexity_budget(&entry.report.analysis);
                entry.report.metrics.file_size as f64 > size_budget
            })
            .count(),
        path_budget_failures: entries
            .iter()
            .filter(|entry| {
                let (_, path_budget) = benchmark_complexity_budget(&entry.report.analysis);
                entry.report.metrics.path_count as f64 > path_budget
            })
            .count(),
    }
}

fn summarize_dataset(entries: &[BenchmarkEntry]) -> BenchmarkDatasetSummary {
    BenchmarkDatasetSummary {
        groups: count_labels(entries.iter().map(|entry| entry.group.clone())),
        subgroups: count_labels(entries.iter().map(|entry| {
            entry
                .reference_subgroup
                .clone()
                .unwrap_or_else(|| "root".to_string())
        })),
        image_kinds: count_labels(
            entries
                .iter()
                .map(|entry| format!("{:?}", entry.report.analysis.image_type).to_lowercase()),
        ),
        complexities: count_labels(
            entries
                .iter()
                .map(|entry| format!("{:?}", entry.report.analysis.complexity).to_lowercase()),
        ),
        kind_complexities: count_labels(entries.iter().map(|entry| {
            format!(
                "{:?}/{:?}",
                entry.report.analysis.image_type, entry.report.analysis.complexity
            )
            .to_lowercase()
        })),
        reference_complexities: count_labels(entries.iter().map(|entry| {
            entry
                .reference_complexity
                .clone()
                .unwrap_or_else(|| format!("{:?}", entry.report.analysis.complexity).to_lowercase())
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
    dataset_health_score: f64,
    balanced_group_score: f64,
    balanced_subgroup_score: f64,
    balanced_kind_complexity_score: f64,
) -> f64 {
    let quality_core = distributions.reliability_score.median * 0.20
        + distributions.reliability_score.p10 * 0.18
        + distributions.reliability_score.min * 0.12
        + distributions.efficiency_score.p10 * 0.08
        + distributions.fidelity_score.p10 * 0.12
        + distributions.local_ssim_p10.p10 * 0.09
        + distributions.edge_f1.p10 * 0.10
        + distributions.color_similarity.p10 * 0.08
        + distributions.topology_score.p10 * 0.05
        + dataset_health_score * 0.05
        + balanced_group_score * 0.05
        + balanced_subgroup_score * 0.04
        + balanced_kind_complexity_score * 0.04;
    let compactness = (1.0 - (distributions.file_size.p90 / 20_000.0)).clamp(0.0, 1.0) * 0.08
        + (1.0 - (distributions.path_count.p90 / 120.0)).clamp(0.0, 1.0) * 0.05;
    let gate_penalty = quality_gates.reliability_below_0_80 as f64 * 0.015
        + quality_gates.fidelity_below_0_75 as f64 * 0.025
        + quality_gates.local_ssim_p10_below_0_75 as f64 * 0.018
        + quality_gates.edge_f1_below_0_75 as f64 * 0.015
        + quality_gates.topology_below_0_50 as f64 * 0.01
        + quality_gates.efficiency_below_0_50 as f64 * 0.008;

    (quality_core + compactness - gate_penalty).clamp(0.0, 1.0)
}

fn dataset_health_score(dataset: &BenchmarkDatasetSummary) -> f64 {
    let group_coverage = (dataset.groups.len() as f64 / 4.0).clamp(0.0, 1.0);
    let complexity_coverage = (dataset.reference_complexities.len() as f64 / 3.0).clamp(0.0, 1.0);
    let kind_complexity_coverage = (dataset.kind_complexities.len() as f64 / 8.0).clamp(0.0, 1.0);
    let subgroup_coverage = (dataset.subgroups.len() as f64 / 8.0).clamp(0.0, 1.0);

    let group_balance = normalized_entropy(&dataset.groups);
    let complexity_balance = normalized_entropy(&dataset.reference_complexities);
    let subgroup_balance = normalized_entropy(&dataset.subgroups);
    let kind_complexity_balance = normalized_entropy(&dataset.kind_complexities);

    (group_coverage * 0.18
        + complexity_coverage * 0.14
        + kind_complexity_coverage * 0.12
        + subgroup_coverage * 0.10
        + group_balance * 0.20
        + complexity_balance * 0.12
        + subgroup_balance * 0.08
        + kind_complexity_balance * 0.06)
        .clamp(0.0, 1.0)
}

fn normalized_entropy(counts: &[BenchmarkDatasetCount]) -> f64 {
    if counts.len() <= 1 {
        return 0.0;
    }

    let total = counts
        .iter()
        .map(|count| count.entries)
        .sum::<usize>()
        .max(1) as f64;
    let entropy = counts
        .iter()
        .map(|count| count.entries as f64 / total)
        .filter(|probability| *probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum::<f64>();
    let max_entropy = (counts.len() as f64).ln().max(1e-6);
    (entropy / max_entropy).clamp(0.0, 1.0)
}

fn reliability_score(metrics: &crate::metrics::QualityMetrics) -> f64 {
    weighted_geometric_mean(&[
        (metrics.fidelity_score, 0.24),
        (metrics.local_ssim_p10, 0.18),
        (metrics.edge_f1, 0.18),
        (metrics.color_similarity, 0.16),
        (metrics.foreground_iou, 0.12),
        (metrics.gradient_similarity, 0.07),
        (metrics.topology_score, 0.05),
    ])
}

fn weighted_geometric_mean(values: &[(f64, f64)]) -> f64 {
    let total_weight = values
        .iter()
        .map(|(_, weight)| *weight)
        .sum::<f64>()
        .max(1.0);
    let sum = values
        .iter()
        .map(|(value, weight)| value.clamp(1e-6, 1.0).ln() * *weight)
        .sum::<f64>();
    (sum / total_weight).exp().clamp(0.0, 1.0)
}

fn balanced_score_by_label(
    entries: &[BenchmarkEntry],
    label_of: impl Fn(&BenchmarkEntry) -> String,
) -> f64 {
    let mut strata = std::collections::BTreeMap::<String, Vec<&BenchmarkEntry>>::new();
    for entry in entries {
        strata.entry(label_of(entry)).or_default().push(entry);
    }
    if strata.is_empty() {
        return 0.0;
    }

    strata
        .values()
        .map(|items| stratum_score(items))
        .sum::<f64>()
        / strata.len() as f64
}

fn stratum_score(entries: &[&BenchmarkEntry]) -> f64 {
    let reliability = distribution(
        entries
            .iter()
            .map(|entry| reliability_score(&entry.report.metrics)),
    );
    let fidelity = distribution(
        entries
            .iter()
            .map(|entry| entry.report.metrics.fidelity_score),
    );
    let edge = distribution(entries.iter().map(|entry| entry.report.metrics.edge_f1));
    let color = distribution(
        entries
            .iter()
            .map(|entry| entry.report.metrics.color_similarity),
    );
    reliability.median * 0.35
        + reliability.p10 * 0.30
        + reliability.min * 0.15
        + fidelity.p10 * 0.10
        + edge.p10 * 0.05
        + color.p10 * 0.05
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

fn benchmark_subgroup(path: &Path, root: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let mut parts = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if parts.len() <= 2 {
        return "root".to_string();
    }
    parts.remove(0);
    let _ = parts.pop();
    if parts.is_empty() {
        "root".to_string()
    } else {
        parts.join("/")
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

#[derive(Debug, Clone)]
struct GoldenSvgInput {
    path: PathBuf,
    group: String,
    subgroup: String,
    source_size_bytes: u64,
    source_element_count: usize,
    reference_complexity: String,
}

fn golden_svg_inputs(dir: &Path) -> Vec<GoldenSvgInput> {
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
        .filter_map(|entry| build_golden_svg_input(dir, entry.into_path()).ok())
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| left.path.cmp(&right.path));
    inputs
}

fn build_golden_svg_input(dir: &Path, path: PathBuf) -> Result<GoldenSvgInput> {
    let source_size_bytes = std::fs::metadata(&path)?.len();
    let source_element_count = std::fs::read_to_string(&path)
        .map(|contents| count_svg_elements(&contents))
        .unwrap_or(0);

    Ok(GoldenSvgInput {
        group: benchmark_group(&path, dir),
        subgroup: benchmark_subgroup(&path, dir),
        reference_complexity: classify_reference_complexity(
            source_size_bytes,
            source_element_count,
        ),
        source_size_bytes,
        source_element_count,
        path,
    })
}

fn count_svg_elements(contents: &str) -> usize {
    [
        "<path",
        "<circle",
        "<rect",
        "<polygon",
        "<polyline",
        "<ellipse",
        "<line",
        "<text",
        "<use",
        "<g",
        "<linearGradient",
        "<radialGradient",
        "<clipPath",
        "<mask",
    ]
    .into_iter()
    .map(|needle| contents.matches(needle).count())
    .sum()
}

fn classify_reference_complexity(source_size_bytes: u64, source_element_count: usize) -> String {
    if source_size_bytes < 2_500 && source_element_count < 12 {
        "simple".to_string()
    } else if source_size_bytes < 12_000 && source_element_count < 64 {
        "medium".to_string()
    } else {
        "complex".to_string()
    }
}

fn select_golden_svg_inputs(dir: &Path, limit: Option<usize>) -> Vec<GoldenSvgInput> {
    let inputs = golden_svg_inputs(dir);
    let Some(limit) = limit else {
        return inputs;
    };
    if limit >= inputs.len() {
        return inputs;
    }

    let mut by_group = std::collections::BTreeMap::<String, Vec<GoldenSvgInput>>::new();
    for input in inputs {
        by_group.entry(input.group.clone()).or_default().push(input);
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
            let has_capacity = by_group
                .get(group)
                .map(|items| !items.is_empty())
                .unwrap_or(false);
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
        selected.extend(select_balanced_group_inputs(items, allocation));
    }
    selected.sort_by(|left, right| left.path.cmp(&right.path));
    selected
}

fn select_balanced_group_inputs(items: &[GoldenSvgInput], count: usize) -> Vec<GoldenSvgInput> {
    if count == 0 {
        return Vec::new();
    }
    if count >= items.len() {
        return items.to_vec();
    }

    let mut by_complexity = std::collections::BTreeMap::<String, Vec<GoldenSvgInput>>::new();
    for item in items {
        by_complexity
            .entry(item.reference_complexity.clone())
            .or_default()
            .push(item.clone());
    }

    let complexity_order = ["simple", "medium", "complex"];
    let active_bins = complexity_order
        .iter()
        .filter(|label| {
            by_complexity
                .get(**label)
                .is_some_and(|items| !items.is_empty())
        })
        .copied()
        .collect::<Vec<_>>();
    if active_bins.is_empty() {
        return sample_evenly(items, count);
    }

    let mut allocations = std::collections::BTreeMap::<String, usize>::new();
    let mut remaining = count;
    if count >= active_bins.len() {
        for label in &active_bins {
            allocations.insert((*label).to_string(), 1);
            remaining = remaining.saturating_sub(1);
        }
    }

    while remaining > 0 {
        let mut progressed = false;
        for label in &active_bins {
            let current = *allocations.get(*label).unwrap_or(&0);
            let capacity = by_complexity
                .get(*label)
                .map(|items| items.len())
                .unwrap_or(0);
            if current < capacity {
                allocations.insert((*label).to_string(), current + 1);
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

    let mut selected = Vec::new();
    for label in &active_bins {
        let Some(bin) = by_complexity.get_mut(*label) else {
            continue;
        };
        bin.sort_by(|left, right| {
            left.subgroup
                .cmp(&right.subgroup)
                .then(left.source_size_bytes.cmp(&right.source_size_bytes))
                .then(left.source_element_count.cmp(&right.source_element_count))
                .then(left.path.cmp(&right.path))
        });
        selected.extend(select_balanced_subgroup_inputs(
            bin,
            *allocations.get(*label).unwrap_or(&0),
        ));
    }

    selected.sort_by(|left, right| left.path.cmp(&right.path));
    selected.truncate(count);
    selected
}

fn select_balanced_subgroup_inputs(items: &[GoldenSvgInput], count: usize) -> Vec<GoldenSvgInput> {
    if count == 0 {
        return Vec::new();
    }
    if count >= items.len() {
        return items.to_vec();
    }

    let mut by_subgroup = std::collections::BTreeMap::<String, Vec<GoldenSvgInput>>::new();
    for item in items {
        by_subgroup
            .entry(item.subgroup.clone())
            .or_default()
            .push(item.clone());
    }
    let subgroups = by_subgroup.keys().cloned().collect::<Vec<_>>();
    if subgroups.is_empty() {
        return sample_evenly(items, count);
    }

    let mut allocations = std::collections::BTreeMap::<String, usize>::new();
    let mut remaining = count;
    if count >= subgroups.len() {
        for subgroup in &subgroups {
            allocations.insert(subgroup.clone(), 1);
            remaining = remaining.saturating_sub(1);
        }
    }

    while remaining > 0 {
        let mut progressed = false;
        for subgroup in &subgroups {
            let current = *allocations.get(subgroup).unwrap_or(&0);
            let capacity = by_subgroup
                .get(subgroup)
                .map(|items| items.len())
                .unwrap_or(0);
            if current < capacity {
                allocations.insert(subgroup.clone(), current + 1);
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

    let mut selected = Vec::new();
    for subgroup in &subgroups {
        let Some(bin) = by_subgroup.get_mut(subgroup) else {
            continue;
        };
        bin.sort_by(|left, right| {
            left.source_size_bytes
                .cmp(&right.source_size_bytes)
                .then(left.source_element_count.cmp(&right.source_element_count))
                .then(left.path.cmp(&right.path))
        });
        selected.extend(sample_evenly(bin, *allocations.get(subgroup).unwrap_or(&0)));
    }

    selected.sort_by(|left, right| left.path.cmp(&right.path));
    selected.truncate(count);
    selected
}

fn sample_evenly<T: Clone>(items: &[T], count: usize) -> Vec<T> {
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

fn write_dataset_manifest(
    golden_dir: &Path,
    reports_dir: &Path,
    inputs: &[GoldenSvgInput],
) -> Result<()> {
    let dataset_dir = reports_dir.join("dataset");
    std::fs::create_dir_all(&dataset_dir)?;
    let manifest = inputs
        .iter()
        .map(|input| {
            let relative = input.path.strip_prefix(golden_dir).unwrap_or(&input.path);
            serde_json::json!({
                "reference": relative.display().to_string(),
                "group": input.group,
                "subgroup": input.subgroup,
                "reference_size_bytes": input.source_size_bytes,
                "reference_element_count": input.source_element_count,
                "reference_complexity": input.reference_complexity,
            })
        })
        .collect::<Vec<_>>();
    std::fs::write(
        reports_dir.join("dataset_manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    std::fs::write(
        dataset_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(())
}

fn write_structured_reports(
    reports_dir: &Path,
    report: &BenchmarkReport,
    dataset_manifest_path: Option<PathBuf>,
) -> Result<()> {
    let summary_dir = reports_dir.join("summary");
    let groups_dir = reports_dir.join("groups");
    let entries_dir = reports_dir.join("entries");
    let failures_dir = reports_dir.join("failures");
    let dataset_dir = reports_dir.join("dataset");
    std::fs::create_dir_all(&summary_dir)?;
    std::fs::create_dir_all(&groups_dir)?;
    std::fs::create_dir_all(entries_dir.join("by_group"))?;
    std::fs::create_dir_all(entries_dir.join("by_subgroup"))?;
    std::fs::create_dir_all(&failures_dir)?;
    std::fs::create_dir_all(failures_dir.join("by_metric"))?;
    std::fs::create_dir_all(&dataset_dir)?;

    std::fs::write(
        summary_dir.join("report.json"),
        serde_json::to_string_pretty(report)?,
    )?;
    std::fs::write(summary_dir.join("report.md"), report.to_markdown())?;
    std::fs::write(
        dataset_dir.join("summary.json"),
        serde_json::to_string_pretty(&report.dataset)?,
    )?;

    for group in &report.groups {
        std::fs::write(
            groups_dir.join(format!("{}.json", group.group)),
            serde_json::to_string_pretty(group)?,
        )?;
        let entries = report
            .entries
            .iter()
            .filter(|entry| entry.group == group.group)
            .collect::<Vec<_>>();
        std::fs::write(
            entries_dir
                .join("by_group")
                .join(format!("{}.json", group.group)),
            serde_json::to_string_pretty(&entries)?,
        )?;
    }
    let mut by_subgroup = std::collections::BTreeMap::<String, Vec<&BenchmarkEntry>>::new();
    for entry in &report.entries {
        by_subgroup
            .entry(
                entry
                    .reference_subgroup
                    .clone()
                    .unwrap_or_else(|| "root".to_string()),
            )
            .or_default()
            .push(entry);
    }
    for (subgroup, entries) in by_subgroup {
        let file_name = subgroup.replace('/', "__");
        std::fs::write(
            entries_dir
                .join("by_subgroup")
                .join(format!("{file_name}.json")),
            serde_json::to_string_pretty(&entries)?,
        )?;
    }
    std::fs::write(
        entries_dir.join("all.json"),
        serde_json::to_string_pretty(&report.entries)?,
    )?;

    let mut worst = report.entries.clone();
    worst.sort_by(|left, right| {
        reliability_score(&left.report.metrics)
            .partial_cmp(&reliability_score(&right.report.metrics))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    worst.truncate(10);
    std::fs::write(
        failures_dir.join("worst_entries.json"),
        serde_json::to_string_pretty(&worst)?,
    )?;
    let mut worst_efficiency = report.entries.clone();
    worst_efficiency.sort_by(|left, right| {
        benchmark_efficiency_score(left)
            .partial_cmp(&benchmark_efficiency_score(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    worst_efficiency.truncate(10);
    std::fs::write(
        failures_dir.join("worst_efficiency.json"),
        serde_json::to_string_pretty(&worst_efficiency)?,
    )?;
    write_metric_failures(
        &failures_dir.join("by_metric"),
        "local_ssim_p10",
        &report.entries,
        |entry| entry.report.metrics.local_ssim_p10,
    )?;
    write_metric_failures(
        &failures_dir.join("by_metric"),
        "edge_f1",
        &report.entries,
        |entry| entry.report.metrics.edge_f1,
    )?;
    write_metric_failures(
        &failures_dir.join("by_metric"),
        "color_similarity",
        &report.entries,
        |entry| entry.report.metrics.color_similarity,
    )?;
    write_metric_failures(
        &failures_dir.join("by_metric"),
        "topology_score",
        &report.entries,
        |entry| entry.report.metrics.topology_score,
    )?;

    if let Some(manifest_path) = dataset_manifest_path {
        if manifest_path.exists() {
            std::fs::copy(manifest_path, dataset_dir.join("manifest.json"))?;
        }
    }

    Ok(())
}

fn write_metric_failures(
    output_dir: &Path,
    metric_name: &str,
    entries: &[BenchmarkEntry],
    metric_of: impl Fn(&BenchmarkEntry) -> f64,
) -> Result<()> {
    let mut worst = entries.to_vec();
    worst.sort_by(|left, right| {
        metric_of(left)
            .partial_cmp(&metric_of(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    worst.truncate(10);
    std::fs::write(
        output_dir.join(format!("{metric_name}.json")),
        serde_json::to_string_pretty(&worst)?,
    )?;
    Ok(())
}

fn benchmark_efficiency_score(entry: &BenchmarkEntry) -> f64 {
    let (size_budget, path_budget) = benchmark_complexity_budget(&entry.report.analysis);
    weighted_geometric_mean(&[
        (
            budget_efficiency(entry.report.metrics.file_size as f64, size_budget),
            0.55,
        ),
        (
            budget_efficiency(entry.report.metrics.path_count as f64, path_budget),
            0.45,
        ),
    ])
}

fn budget_efficiency(actual: f64, budget: f64) -> f64 {
    let ratio = actual / budget.max(1.0);
    if ratio <= 1.0 {
        1.0
    } else {
        (1.0 / (1.0 + (ratio - 1.0) * 0.85)).clamp(0.0, 1.0)
    }
}

fn benchmark_complexity_budget(analysis: &ImageAnalysis) -> (f64, f64) {
    let base = match (analysis.image_type, analysis.complexity) {
        (ImageKind::Icon, Complexity::Simple) => (1_200.0, 8.0),
        (ImageKind::Icon, Complexity::Medium) => (2_000.0, 14.0),
        (ImageKind::Icon, Complexity::Complex) => (3_000.0, 24.0),
        (ImageKind::Logo, Complexity::Simple) => (2_400.0, 12.0),
        (ImageKind::Logo, Complexity::Medium) => (4_000.0, 20.0),
        (ImageKind::Logo, Complexity::Complex) => (6_000.0, 32.0),
        (ImageKind::Illustration, Complexity::Simple) => (8_000.0, 40.0),
        (ImageKind::Illustration, Complexity::Medium) => (14_000.0, 88.0),
        (ImageKind::Illustration, Complexity::Complex) => (20_000.0, 132.0),
        (ImageKind::Photo, Complexity::Simple) => (12_000.0, 48.0),
        (ImageKind::Photo, Complexity::Medium) => (18_000.0, 96.0),
        (ImageKind::Photo, Complexity::Complex) => (24_000.0, 144.0),
    };
    let pixels = (analysis.width as f64 * analysis.height as f64).max(1.0);
    let resolution_scale = (pixels / (1024.0 * 1024.0)).sqrt().clamp(0.5, 2.5);
    let color_scale = match analysis.image_type {
        ImageKind::Icon | ImageKind::Logo => ((analysis.effective_colors as f64).max(1.0) / 12.0)
            .sqrt()
            .clamp(0.8, 2.0),
        ImageKind::Illustration => ((analysis.effective_colors as f64).max(1.0) / 48.0)
            .sqrt()
            .clamp(0.8, 2.4),
        ImageKind::Photo => ((analysis.effective_colors as f64).max(1.0) / 96.0)
            .sqrt()
            .clamp(0.9, 2.8),
    };
    let edge_scale = (1.0 + analysis.edge_density * 6.0).clamp(1.0, 1.8);

    (
        base.0 * resolution_scale * color_scale,
        base.1 * resolution_scale * color_scale.min(1.6) * edge_scale,
    )
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
            .map(|input| {
                input
                    .path
                    .strip_prefix(dir.path())
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
        assert_eq!(
            labels
                .iter()
                .filter(|label| *label == "illustrations")
                .count(),
            2
        );
    }
}
