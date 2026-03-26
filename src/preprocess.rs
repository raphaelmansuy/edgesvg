use anyhow::Result;
use exoquant::{convert_to_indexed, ditherer, optimizer, Color};
use image::{DynamicImage, Rgba, RgbaImage};

use crate::types::{ImageAnalysis, ImageKind, QualityPreset, TraceMode, TraceSettings};

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
    let _ = analysis;
    trace_settings_for_preset(quality)
}

pub fn trace_settings_for_preset(quality: QualityPreset) -> TraceSettings {
    match quality {
        QualityPreset::Figma => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 4,
            color_precision: 6,
            layer_difference: 16,
            length_threshold: 4.0,
            corner_threshold: 60,
            max_iterations: 10,
            splice_threshold: 45,
            path_precision: 5,
            optimizer_precision: 1,
        },
        QualityPreset::Balanced => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 2,
            color_precision: 6,
            layer_difference: 8,
            length_threshold: 3.5,
            corner_threshold: 45,
            max_iterations: 15,
            splice_threshold: 45,
            path_precision: 6,
            optimizer_precision: 2,
        },
        QualityPreset::Quality => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 1,
            color_precision: 8,
            layer_difference: 4,
            length_threshold: 3.0,
            corner_threshold: 30,
            max_iterations: 20,
            splice_threshold: 45,
            path_precision: 8,
            optimizer_precision: 2,
        },
        QualityPreset::Ultra => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Polygon,
            filter_speckle: 0,
            color_precision: 8,
            layer_difference: 0,
            length_threshold: 3.5,
            corner_threshold: 10,
            max_iterations: 50,
            splice_threshold: 45,
            path_precision: 10,
            optimizer_precision: 2,
        },
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

#[cfg(test)]
mod tests {
    use super::trace_settings_for_preset;
    use crate::types::{QualityPreset, TraceMode};

    #[test]
    fn vectalab_hifi_presets_match_reference_values() {
        let figma = trace_settings_for_preset(QualityPreset::Figma);
        assert_eq!(figma.mode, TraceMode::Spline);
        assert_eq!(figma.filter_speckle, 4);
        assert_eq!(figma.path_precision, 5);
        assert_eq!(figma.optimizer_precision, 1);

        let balanced = trace_settings_for_preset(QualityPreset::Balanced);
        assert_eq!(balanced.filter_speckle, 2);
        assert_eq!(balanced.layer_difference, 8);
        assert_eq!(balanced.max_iterations, 15);

        let quality = trace_settings_for_preset(QualityPreset::Quality);
        assert_eq!(quality.color_precision, 8);
        assert_eq!(quality.layer_difference, 4);
        assert_eq!(quality.path_precision, 8);

        let ultra = trace_settings_for_preset(QualityPreset::Ultra);
        assert_eq!(ultra.mode, TraceMode::Polygon);
        assert_eq!(ultra.filter_speckle, 0);
        assert_eq!(ultra.layer_difference, 0);
        assert_eq!(ultra.max_iterations, 50);
        assert_eq!(ultra.path_precision, 10);
    }
}
