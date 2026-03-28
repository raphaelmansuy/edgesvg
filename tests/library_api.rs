use std::path::Path;

use edgesvg::{
    analyze_image, benchmark_directory, benchmark_golden_data, compute_metrics,
    determine_auto_mode, optimize, preprocess_image, quantize_image, render_png, vectorize,
    vectorize_auto, vectorize_logo_premium, vectorize_optimal, vectorize_path, vectorize_premium,
    vectorize_smart, AutoMode, LogoQualityPreset, QualityPreset, VectorizeMethod, VectorizeOptions,
    VectorizeRequest,
};
use image::{DynamicImage, Rgba, RgbaImage};
use tempfile::tempdir;

fn sample_image() -> DynamicImage {
    let mut image = RgbaImage::new(96, 96);
    for y in 0..96 {
        for x in 0..96 {
            let pixel = if x < 32 {
                Rgba([220, 40, 40, 255])
            } else if x < 64 {
                Rgba([40, 160, 220, 255])
            } else {
                Rgba([250, 250, 250, 255])
            };
            image.put_pixel(x, y, pixel);
        }
    }
    DynamicImage::ImageRgba8(image)
}

#[test]
fn library_surface_handles_basic_vectorization_flow() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.png");
    sample_image().save(&input).unwrap();

    let analysis = analyze_image(&sample_image());
    assert_eq!(analysis.width, 96);

    let processed = preprocess_image(&sample_image(), &analysis, Some(8)).unwrap();
    assert_eq!(processed.dimensions(), (96, 96));

    let quantized = quantize_image(&processed, 2);
    let colors = quantized
        .pixels()
        .map(|p| p.0)
        .collect::<std::collections::HashSet<_>>();
    assert!(colors.len() <= 2);

    let (svg, report) = vectorize(
        &input,
        &VectorizeOptions {
            target_ssim: 0.5,
            max_file_size: 200_000,
            max_iterations: 2,
            quality: Some(QualityPreset::Balanced),
        },
    )
    .unwrap();

    assert!(svg.contains("<svg"));
    assert!(report.metrics.path_count > 0);

    let metrics = compute_metrics(&image::open(Path::new(&input)).unwrap(), &svg).unwrap();
    assert!(metrics.ssim >= 0.0);
    assert!(metrics.file_size > 0);
}

#[test]
fn benchmark_runner_produces_entries_and_markdown() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().join("inputs");
    let output_dir = dir.path().join("outputs");
    std::fs::create_dir_all(&input_dir).unwrap();
    sample_image().save(input_dir.join("sample.png")).unwrap();

    let report =
        benchmark_directory(&input_dir, &output_dir, &VectorizeOptions::default()).unwrap();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.groups.len(), 1);
    assert!(report.average_elapsed_ms >= 0.0);
    assert!(report.to_markdown().contains("Benchmark Report"));
    assert!(report.to_markdown().contains("By Group"));
}

#[test]
fn golden_benchmark_rasterizes_reference_svgs() {
    let dir = tempdir().unwrap();
    let golden_dir = dir.path().join("golden");
    let work_dir = dir.path().join("work");
    std::fs::create_dir_all(&golden_dir).unwrap();
    std::fs::write(
        golden_dir.join("sample.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="16" height="32" fill="#ff0000"/><rect x="16" width="16" height="32" fill="#0000ff"/></svg>"##,
    )
    .unwrap();

    let report = benchmark_golden_data(
        &golden_dir,
        &work_dir,
        &VectorizeOptions {
            target_ssim: 0.5,
            max_file_size: 200_000,
            max_iterations: 1,
            quality: Some(QualityPreset::Figma),
        },
        None,
    )
    .unwrap();

    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].group, "sample.svg");
    assert_eq!(report.entries[0].reference.as_deref(), Some("sample.svg"));
    assert!(report.robust_benchmark_score >= 0.0);
    assert!(work_dir.join("artifacts/rendered_inputs/sample.png").exists());
    assert!(work_dir.join("artifacts/vectorized/sample.svg").exists());
    assert!(work_dir.join("reports/dataset_manifest.json").exists());
}

#[test]
fn higher_level_api_handles_logo_premium_and_auto_modes() {
    let dir = tempdir().unwrap();
    let logo = dir.path().join("logo.png");
    let icon = dir.path().join("icon.png");
    sample_image().save(&logo).unwrap();

    let mut icon_image = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let pixel = if x > 6 && x < 26 && y > 6 && y < 26 {
                Rgba([30, 40, 50, 255])
            } else {
                Rgba([0, 0, 0, 0])
            };
            icon_image.put_pixel(x, y, pixel);
        }
    }
    DynamicImage::ImageRgba8(icon_image).save(&icon).unwrap();

    let (logo_svg, logo_report) =
        vectorize_logo_premium(&logo, LogoQualityPreset::Clean, Some(3)).unwrap();
    assert!(logo_svg.contains("<svg"));
    assert!(logo_report.metrics.path_count > 0);

    let (premium_svg, premium_report) = vectorize_premium(&logo, 0.9, Some(3)).unwrap();
    assert!(premium_svg.contains("<svg"));
    assert!(premium_report.metrics.ssim >= 0.0);

    let decision = determine_auto_mode(&icon).unwrap();
    assert_eq!(decision.mode, AutoMode::Logo);

    let (auto_svg, auto_report) = vectorize_auto(&icon).unwrap();
    assert!(auto_svg.contains("<svg"));
    assert!(auto_report.metrics.path_count > 0);

    let (smart_svg, smart_report) = vectorize_smart(&logo, 0.9, 100_000, 3).unwrap();
    assert!(smart_svg.contains("<svg"));
    assert!(smart_report.metrics.ssim >= 0.0);

    let (optimal_svg, optimal_report) = vectorize_optimal(&logo).unwrap();
    assert!(optimal_svg.contains("<svg"));
    assert!(optimal_report.metrics.path_count > 0);
}

#[test]
fn svg_optimizer_keeps_invalid_input_unchanged() {
    assert_eq!(edgesvg::optimize_svg("not valid svg", 1), "not valid svg");
}

#[test]
fn stable_sdk_contract_handles_path_and_serializable_outputs() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.png");
    sample_image().save(&input).unwrap();

    let response = vectorize_path(
        &input,
        &VectorizeRequest {
            method: VectorizeMethod::Auto,
            ..VectorizeRequest::default()
        },
    )
    .unwrap();

    assert!(response.svg.contains("<svg"));
    assert!(response.report.metrics.ssim >= 0.0);

    let optimized = optimize(&response.svg, 2);
    assert!(optimized.optimized_svg.contains("<svg"));

    let png = render_png(&response.svg, 96, 96).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
}
