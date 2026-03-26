use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use edgesvg::metrics::render_svg_file_to_png;
use edgesvg::{
    analyze_image, benchmark_directory, benchmark_golden_data, compute_metrics,
    determine_auto_mode, vectorize, vectorize_auto, vectorize_logo_premium, vectorize_optimal,
    vectorize_premium, vectorize_smart, write_svg, LogoQualityPreset, QualityPreset,
    VectorizeOptions,
};
use serde::Serialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum ConvertMethod {
    Hifi,
    Logo,
    Premium,
    Auto,
    Smart,
    Optimal,
    Bayesian,
    Sam,
}

fn logo_quality_from_quality(quality: QualityPreset) -> LogoQualityPreset {
    match quality {
        QualityPreset::Figma => LogoQualityPreset::Clean,
        QualityPreset::Balanced => LogoQualityPreset::Balanced,
        QualityPreset::Quality => LogoQualityPreset::High,
        QualityPreset::Ultra => LogoQualityPreset::Ultra,
    }
}

fn file_size_label(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn optimize_svg_file(
    input: &PathBuf,
    output: Option<PathBuf>,
    precision: u32,
    json_output: bool,
) -> Result<()> {
    if !input
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        anyhow::bail!("optimize expects an .svg input");
    }

    let output = output.unwrap_or_else(|| input.clone());
    let original = std::fs::read_to_string(input)?;
    let original_size = original.len();
    let optimized = edgesvg::optimize_svg(&original, precision);
    let optimized_size = optimized.len();
    let reduction_percent = if original_size == 0 {
        0.0
    } else {
        (1.0 - optimized_size as f64 / original_size as f64) * 100.0
    };

    std::fs::write(&output, optimized)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "input": input,
                "output": output,
                "precision": precision,
                "original_size": original_size,
                "optimized_size": optimized_size,
                "reduction_percent": reduction_percent
            }))?
        );
    } else {
        println!(
            "wrote {} | precision={} original={} optimized={} reduction={:.1}%",
            output.display(),
            precision,
            original_size,
            optimized_size,
            reduction_percent
        );
    }

    Ok(())
}

fn print_info(input: &PathBuf, json_output: bool) -> Result<()> {
    let image = image::open(input)?;
    let analysis = analyze_image(&image);
    let metadata = std::fs::metadata(input)?;
    let color = image.color();
    let channels = color.channel_count();
    let color_mode = match channels {
        1 => "grayscale",
        3 => "rgb",
        4 => "rgba",
        _ => "unknown",
    };
    let recommended_method = if analysis.width.max(analysis.height) <= 512 {
        ConvertMethod::Hifi
    } else {
        ConvertMethod::Premium
    };
    let recommended_quality = if analysis.width.max(analysis.height) <= 512 {
        QualityPreset::Ultra
    } else {
        QualityPreset::Balanced
    };
    let recommended_target = if analysis.width.max(analysis.height) <= 512 {
        0.998
    } else {
        0.995
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "path": input,
                "file_size_bytes": metadata.len(),
                "file_size": file_size_label(metadata.len()),
                "format": input.extension().and_then(|ext| ext.to_str()).unwrap_or("unknown"),
                "channels": channels,
                "color_mode": color_mode,
                "analysis": analysis,
                "recommended": {
                    "method": recommended_method,
                    "quality": recommended_quality,
                    "target_ssim": recommended_target
                }
            }))?
        );
    } else {
        println!(
            "{} | {}x{} {} channels={} type={:?} complexity={:?} unique_colors={} edge_density={:.4} recommend={} quality={:?} target={:.3}",
            input.display(),
            analysis.width,
            analysis.height,
            file_size_label(metadata.len()),
            channels,
            analysis.image_type,
            analysis.complexity,
            analysis.unique_colors,
            analysis.edge_density,
            format!("{recommended_method:?}").to_lowercase(),
            recommended_quality,
            recommended_target
        );
    }

    Ok(())
}

#[derive(Parser)]
#[command(
    name = "edgesvg",
    version,
    about = "Native Rust raster-to-SVG vectorization"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(visible_alias = "hifi")]
    Convert {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long = "target", alias = "target-ssim", default_value_t = 0.998)]
        target_ssim: f64,
        #[arg(long, default_value_t = 100_000)]
        max_file_size: usize,
        #[arg(long, default_value_t = 4)]
        max_iterations: usize,
        #[arg(long, value_enum, default_value_t = QualityPreset::Ultra)]
        quality: QualityPreset,
        #[arg(long, short = 'm', value_enum, default_value_t = ConvertMethod::Hifi)]
        method: ConvertMethod,
        #[arg(long)]
        colors: Option<usize>,
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
    #[command(visible_alias = "optimal")]
    Premium {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long = "target", alias = "target-ssim", default_value_t = 0.98)]
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
    Smart {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long = "quality", alias = "target-ssim", default_value_t = 0.92)]
        target_ssim: f64,
        #[arg(long = "size", default_value_t = 100)]
        target_size_kb: usize,
        #[arg(long = "iterations", default_value_t = 5)]
        max_iterations: usize,
        #[arg(long)]
        json: bool,
    },
    Info {
        input: PathBuf,
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
    Optimize {
        input: PathBuf,
        output: Option<PathBuf>,
        #[arg(long, short = 'p', default_value_t = 2)]
        precision: u32,
        #[arg(long)]
        json: bool,
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
        #[arg(long, default_value_t = 4)]
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
        #[arg(long, default_value_t = 4)]
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
            method,
            colors,
            json,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("svg"));
            let (svg, report, fallback_from) = match method {
                ConvertMethod::Hifi => {
                    let options = VectorizeOptions {
                        target_ssim,
                        max_file_size,
                        max_iterations,
                        quality: Some(quality),
                    };
                    let (svg, report) = vectorize(&input, &options)?;
                    (svg, report, None)
                }
                ConvertMethod::Smart => {
                    let (svg, report) =
                        vectorize_smart(&input, target_ssim, max_file_size, max_iterations)?;
                    (svg, report, None)
                }
                ConvertMethod::Logo => {
                    let (svg, report) =
                        vectorize_logo_premium(&input, logo_quality_from_quality(quality), colors)?;
                    (svg, report, None)
                }
                ConvertMethod::Premium => {
                    let (svg, report) = vectorize_premium(&input, target_ssim, colors)?;
                    (svg, report, None)
                }
                ConvertMethod::Optimal => {
                    let (svg, report) = vectorize_optimal(&input)?;
                    (svg, report, None)
                }
                ConvertMethod::Auto => {
                    let (svg, report) = vectorize_auto(&input)?;
                    (svg, report, None)
                }
                ConvertMethod::Bayesian => {
                    let (svg, report) = vectorize_smart(
                        &input,
                        target_ssim.max(0.95),
                        max_file_size,
                        max_iterations.max(5),
                    )?;
                    (svg, report, None)
                }
                ConvertMethod::Sam => {
                    let (svg, report) = vectorize_auto(&input)?;
                    (svg, report, Some(method))
                }
            };
            write_svg(&output, &svg)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "requested_method": method,
                        "fallback_from": fallback_from,
                        "report": report
                    }))?
                );
            } else {
                let fallback_note = fallback_from
                    .map(|fallback| {
                        format!(" fallback_from={}", format!("{fallback:?}").to_lowercase())
                    })
                    .unwrap_or_default();
                println!(
                    "wrote {} | method={} type={:?} preset={:?} ssim={:.4} size={:.1}KB paths={}{}",
                    output.display(),
                    format!("{method:?}").to_lowercase(),
                    report.analysis.image_type,
                    report.quality_preset,
                    report.metrics.ssim,
                    report.metrics.file_size as f64 / 1024.0,
                    report.metrics.path_count,
                    fallback_note
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
        Commands::Smart {
            input,
            output,
            target_ssim,
            target_size_kb,
            max_iterations,
            json,
        } => {
            let output = output.unwrap_or_else(|| input.with_extension("svg"));
            let (svg, report) =
                vectorize_smart(&input, target_ssim, target_size_kb * 1024, max_iterations)?;
            write_svg(&output, &svg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "wrote {} | mode=smart ssim={:.4} size={:.1}KB paths={}",
                    output.display(),
                    report.metrics.ssim,
                    report.metrics.file_size as f64 / 1024.0,
                    report.metrics.path_count
                );
            }
        }
        Commands::Info { input, json } => {
            print_info(&input, json)?;
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
        Commands::Optimize {
            input,
            output,
            precision,
            json,
        } => {
            optimize_svg_file(&input, output, precision, json)?;
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
