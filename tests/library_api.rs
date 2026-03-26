use std::path::Path;

use image::{DynamicImage, Rgba, RgbaImage};
use tempfile::tempdir;
use vectalab::{
    analyze_image, benchmark_directory, compute_metrics, optimize_svg, preprocess_image,
    quantize_image, vectorize, QualityPreset, VectorizeOptions,
};

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
    assert!(report.to_markdown().contains("Benchmark Report"));
}

#[test]
fn svg_optimizer_keeps_invalid_input_unchanged() {
    assert_eq!(optimize_svg("not valid svg", 1), "not valid svg");
}
