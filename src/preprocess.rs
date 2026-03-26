use anyhow::Result;
use exoquant::{convert_to_indexed, ditherer, optimizer, Color};
use image::{DynamicImage, Rgba, RgbaImage};
use visioncortex::PathSimplifyMode;
use vtracer::Config;

use crate::types::{ImageAnalysis, ImageKind, QualityPreset, TraceSettings};

pub fn preprocess_image(
    image: &DynamicImage,
    analysis: &ImageAnalysis,
    palette_override: Option<usize>,
) -> Result<RgbaImage> {
    let mut processed = image.to_rgba8();
    let target_colors = palette_override.unwrap_or_else(|| default_palette_size(analysis));

    processed = match analysis.image_type {
        ImageKind::Logo => quantize_image(&processed, target_colors.clamp(4, 16)),
        ImageKind::Icon => quantize_image(&processed, target_colors.clamp(8, 32)),
        ImageKind::Illustration => quantize_image(&processed, target_colors.clamp(12, 64)),
        ImageKind::Photo => DynamicImage::ImageRgba8(processed).blur(0.8).to_rgba8(),
    };

    if matches!(analysis.image_type, ImageKind::Logo | ImageKind::Icon) {
        processed = DynamicImage::ImageRgba8(processed).blur(0.35).to_rgba8();
    }

    Ok(processed)
}

pub fn quantize_image(image: &RgbaImage, n_colors: usize) -> RgbaImage {
    let pixels = image
        .pixels()
        .map(|p| Color::new(p[0], p[1], p[2], p[3]))
        .collect::<Vec<_>>();
    let (palette, indexed) = convert_to_indexed(
        &pixels,
        image.width() as usize,
        n_colors.min(256),
        &optimizer::KMeans,
        &ditherer::None,
    );

    let mut out = RgbaImage::new(image.width(), image.height());
    for (pixel, index) in out.pixels_mut().zip(indexed.into_iter()) {
        let color = palette[index as usize];
        *pixel = Rgba([color.r, color.g, color.b, color.a]);
    }
    out
}

pub fn count_unique_colors(image: &RgbaImage) -> usize {
    image
        .pixels()
        .map(|pixel| pixel.0)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

pub fn adaptive_trace_settings(analysis: &ImageAnalysis, quality: QualityPreset) -> TraceSettings {
    let mut base = match analysis.image_type {
        ImageKind::Logo => TraceSettings {
            filter_speckle: 8,
            color_precision: 4,
            layer_difference: 32,
            corner_threshold: 60,
            length_threshold: 4.0,
            max_iterations: 10,
            splice_threshold: 45,
            path_precision: 3,
        },
        ImageKind::Icon => TraceSettings {
            filter_speckle: 6,
            color_precision: 5,
            layer_difference: 24,
            corner_threshold: 50,
            length_threshold: 3.5,
            max_iterations: 12,
            splice_threshold: 45,
            path_precision: 4,
        },
        ImageKind::Illustration => TraceSettings {
            filter_speckle: 4,
            color_precision: 6,
            layer_difference: 16,
            corner_threshold: 45,
            length_threshold: 3.0,
            max_iterations: 15,
            splice_threshold: 45,
            path_precision: 5,
        },
        ImageKind::Photo => TraceSettings {
            filter_speckle: 2,
            color_precision: 6,
            layer_difference: 8,
            corner_threshold: 30,
            length_threshold: 3.0,
            max_iterations: 20,
            splice_threshold: 45,
            path_precision: 6,
        },
    };

    match quality {
        QualityPreset::Compact => {
            base.filter_speckle = (base.filter_speckle * 2).min(16);
            base.layer_difference = (base.layer_difference * 2).min(64);
            base.color_precision = (base.color_precision - 2).max(3);
            base.path_precision = base.path_precision.saturating_sub(1).max(2);
        }
        QualityPreset::Balanced => {}
        QualityPreset::Quality => {
            base.filter_speckle = (base.filter_speckle / 2).max(1);
            base.layer_difference = (base.layer_difference / 2).max(4);
            base.color_precision = (base.color_precision + 1).min(8);
            base.path_precision += 1;
        }
        QualityPreset::Ultra => {
            base.filter_speckle = (base.filter_speckle / 2).max(1);
            base.layer_difference = (base.layer_difference / 2).max(4);
            base.color_precision = (base.color_precision + 2).min(8);
            base.path_precision += 2;
            base.max_iterations += 4;
            base.length_threshold = base.length_threshold.max(2.5);
        }
    }

    base
}

pub fn to_vtracer_config(settings: &TraceSettings) -> Config {
    Config {
        color_mode: vtracer::ColorMode::Color,
        hierarchical: vtracer::Hierarchical::Stacked,
        filter_speckle: settings.filter_speckle,
        color_precision: settings.color_precision,
        layer_difference: settings.layer_difference,
        mode: PathSimplifyMode::Spline,
        corner_threshold: settings.corner_threshold,
        length_threshold: settings.length_threshold,
        max_iterations: settings.max_iterations,
        splice_threshold: settings.splice_threshold,
        path_precision: Some(settings.path_precision),
    }
}

fn default_palette_size(analysis: &ImageAnalysis) -> usize {
    match analysis.image_type {
        ImageKind::Logo => {
            if analysis.top_10_coverage > 0.95 {
                8
            } else if analysis.top_10_coverage > 0.90 {
                12
            } else {
                16
            }
        }
        ImageKind::Icon => 24,
        ImageKind::Illustration => 48,
        ImageKind::Photo => 32,
    }
}
