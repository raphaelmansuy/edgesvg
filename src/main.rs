use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use vectalab::metrics::render_svg_file_to_png;
use vectalab::{
    analyze_image, benchmark_directory, compute_metrics, vectorize, write_svg, QualityPreset,
    VectorizeOptions,
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
        #[arg(long, default_value_t = 0.92)]
        target_ssim: f64,
        #[arg(long, default_value_t = 100_000)]
        max_file_size: usize,
        #[arg(long, default_value_t = 5)]
        max_iterations: usize,
        #[arg(long, value_enum)]
        quality: Option<QualityPreset>,
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
        #[arg(long, default_value_t = 0.92)]
        target_ssim: f64,
        #[arg(long, default_value_t = 100_000)]
        max_file_size: usize,
        #[arg(long, default_value_t = 5)]
        max_iterations: usize,
        #[arg(long, value_enum)]
        quality: Option<QualityPreset>,
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
                quality,
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
                quality,
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
    }

    Ok(())
}
