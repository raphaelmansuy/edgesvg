use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use vectalab::metrics::render_svg_file_to_png;
use vectalab::{
    analyze_image, benchmark_directory, benchmark_golden_data, compute_metrics,
    determine_auto_mode, vectorize, vectorize_auto, vectorize_logo_premium, vectorize_premium,
    write_svg, LogoQualityPreset, QualityPreset, VectorizeOptions,
};

#[derive(Parser)]
#[command(
    name = "vectalab",
    version,
    about = "Native Rust raster-to-SVG vectorization"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Convert {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, default_value_t = 0.998)]
        target_ssim: f64,
        #[arg(long, default_value_t = 100_000)]
        max_file_size: usize,
        #[arg(long, default_value_t = 1)]
        max_iterations: usize,
        #[arg(long, value_enum, default_value_t = QualityPreset::Ultra)]
        quality: QualityPreset,
        #[arg(long)]
        json: bool,
    },
    Logo {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = LogoQualityPreset::Balanced)]
        quality: LogoQualityPreset,
        #[arg(long)]
        colors: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    Premium {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, default_value_t = 0.98)]
        target_ssim: f64,
        #[arg(long)]
        colors: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    Auto {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Analyze {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Compare {
        input: PathBuf,
        svg: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Render {
        input: PathBuf,
        output: PathBuf,
    },
    Benchmark {
        #[arg(long)]
        input_dir: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
        #[arg(long, default_value_t = 0.998)]
        target_ssim: f64,
        #[arg(long, default_value_t = 100_000)]
        max_file_size: usize,
        #[arg(long, default_value_t = 1)]
        max_iterations: usize,
        #[arg(long, value_enum, default_value_t = QualityPreset::Ultra)]
        quality: QualityPreset,
        #[arg(long)]
        json_path: Option<PathBuf>,
        #[arg(long)]
        markdown_path: Option<PathBuf>,
    },
    BenchmarkGolden {
        #[arg(long, default_value = "golden_data")]
        golden_dir: PathBuf,
        #[arg(long, default_value = "benchmark_runs/golden_data")]
        work_dir: PathBuf,
        #[arg(long, default_value_t = 0.998)]
        target_ssim: f64,
        #[arg(long, default_value_t = 100_000)]
        max_file_size: usize,
        #[arg(long, default_value_t = 1)]
        max_iterations: usize,
        #[arg(long, value_enum, default_value_t = QualityPreset::Figma)]
        quality: QualityPreset,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        json_path: Option<PathBuf>,
        #[arg(long)]
        markdown_path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Convert {
            input,
            output,
            target_ssim,
            max_file_size,
            max_iterations,
            quality,
            json,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("svg"));
            let options = VectorizeOptions {
                target_ssim,
                max_file_size,
                max_iterations,
                quality: Some(quality),
            };
            let (svg, report) = vectorize(&input, &options)?;
            write_svg(&output, &svg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "wrote {} | type={:?} preset={:?} ssim={:.4} size={:.1}KB paths={}",
                    output.display(),
                    report.analysis.image_type,
                    report.quality_preset,
                    report.metrics.ssim,
                    report.metrics.file_size as f64 / 1024.0,
                    report.metrics.path_count
                );
            }
        }
        Commands::Logo {
            input,
            output,
            quality,
            colors,
            json,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("svg"));
            let (svg, report) = vectorize_logo_premium(&input, quality, colors)?;
            write_svg(&output, &svg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "wrote {} | mode=logo preset={:?} ssim={:.4} size={:.1}KB paths={}",
                    output.display(),
                    quality,
                    report.metrics.ssim,
                    report.metrics.file_size as f64 / 1024.0,
                    report.metrics.path_count
                );
            }
        }
        Commands::Premium {
            input,
            output,
            target_ssim,
            colors,
            json,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("svg"));
            let (svg, report) = vectorize_premium(&input, target_ssim, colors)?;
            write_svg(&output, &svg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "wrote {} | mode=premium ssim={:.4} size={:.1}KB paths={}",
                    output.display(),
                    report.metrics.ssim,
                    report.metrics.file_size as f64 / 1024.0,
                    report.metrics.path_count
                );
            }
        }
        Commands::Auto {
            input,
            output,
            json,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("svg"));
            let decision = determine_auto_mode(&input)?;
            let (svg, report) = vectorize_auto(&input)?;
            write_svg(&output, &svg)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "decision": decision,
                        "report": report
                    }))?
                );
            } else {
                println!(
                    "wrote {} | mode={:?} ssim={:.4} size={:.1}KB paths={}",
                    output.display(),
                    decision.mode,
                    report.metrics.ssim,
                    report.metrics.file_size as f64 / 1024.0,
                    report.metrics.path_count
                );
            }
        }
        Commands::Analyze { input, json } => {
            let image = image::open(&input)?;
            let analysis = analyze_image(&image);
            if json {
                println!("{}", serde_json::to_string_pretty(&analysis)?);
            } else {
                println!(
                    "{}x{} {:?} {:?} unique_colors={} edge_density={:.4}",
                    analysis.width,
                    analysis.height,
                    analysis.image_type,
                    analysis.complexity,
                    analysis.unique_colors,
                    analysis.edge_density
                );
            }
        }
        Commands::Compare { input, svg, json } => {
            let original = image::open(&input)?;
            let svg_content = std::fs::read_to_string(&svg)?;
            let metrics = compute_metrics(&original, &svg_content)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&metrics)?);
            } else {
                println!(
                    "ssim={:.4} psnr={:.2} mae={:.2} size={:.1}KB paths={}",
                    metrics.ssim,
                    metrics.psnr,
                    metrics.mae,
                    metrics.file_size as f64 / 1024.0,
                    metrics.path_count
                );
            }
        }
        Commands::Render { input, output } => {
            render_svg_file_to_png(&input, &output)?;
            println!("wrote {}", output.display());
        }
        Commands::Benchmark {
            input_dir,
            output_dir,
            target_ssim,
            max_file_size,
            max_iterations,
            quality,
            json_path,
            markdown_path,
        } => {
            let options = VectorizeOptions {
                target_ssim,
                max_file_size,
                max_iterations,
                quality: Some(quality),
            };
            let report = benchmark_directory(&input_dir, &output_dir, &options)?;
            if let Some(path) = json_path {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
            }
            if let Some(path) = markdown_path {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, report.to_markdown())?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "entries": report.entries.len(),
                    "average_ssim": report.average_ssim,
                    "average_psnr": report.average_psnr,
                    "average_mae": report.average_mae,
                    "average_file_size": report.average_file_size,
                    "average_path_count": report.average_path_count
                }))?
            );
        }
        Commands::BenchmarkGolden {
            golden_dir,
            work_dir,
            target_ssim,
            max_file_size,
            max_iterations,
            quality,
            limit,
            json_path,
            markdown_path,
        } => {
            let options = VectorizeOptions {
                target_ssim,
                max_file_size,
                max_iterations,
                quality: Some(quality),
            };
            let report = benchmark_golden_data(&golden_dir, &work_dir, &options, limit)?;
            if let Some(path) = json_path {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
            }
            if let Some(path) = markdown_path {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, report.to_markdown())?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "entries": report.entries.len(),
                    "average_ssim": report.average_ssim,
                    "average_psnr": report.average_psnr,
                    "average_mae": report.average_mae,
                    "average_file_size": report.average_file_size,
                    "average_path_count": report.average_path_count
                }))?
            );
        }
    }

    Ok(())
}
