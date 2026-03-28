use anyhow::Result;
use exoquant::{convert_to_indexed, ditherer, optimizer, Color};
use image::{DynamicImage, Rgba, RgbaImage};

use crate::types::{
    ImageAnalysis, ImageKind, LogoQualityPreset, QualityPreset, TraceMode, TraceSettings,
};

pub const MONOCHROME_VISIBLE_ALPHA_THRESHOLD: u8 = 16;
pub const MONOCHROME_MASK_ALPHA_THRESHOLD: u8 = 128;

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

pub fn detect_sparse_monochrome_color(image: &RgbaImage) -> Option<[u8; 3]> {
    let total = (image.width() as usize * image.height() as usize).max(1);
    let mut visible = 0usize;
    let mut weight_sum = 0.0;
    let mut weighted_mean = [0.0; 3];

    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < MONOCHROME_VISIBLE_ALPHA_THRESHOLD {
            continue;
        }

        visible += 1;
        let weight = f64::from(a) / 255.0;
        weight_sum += weight;
        weighted_mean[0] += f64::from(r) * weight;
        weighted_mean[1] += f64::from(g) * weight;
        weighted_mean[2] += f64::from(b) * weight;
    }

    if visible == 0 || weight_sum <= 0.0 {
        return None;
    }

    let coverage = visible as f64 / total as f64;
    if !(0.01..0.95).contains(&coverage) {
        return None;
    }

    for channel in &mut weighted_mean {
        *channel /= weight_sum;
    }

    let mut variance = [0.0; 3];
    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < MONOCHROME_VISIBLE_ALPHA_THRESHOLD {
            continue;
        }

        let weight = f64::from(a) / 255.0;
        variance[0] += (f64::from(r) - weighted_mean[0]).powi(2) * weight;
        variance[1] += (f64::from(g) - weighted_mean[1]).powi(2) * weight;
        variance[2] += (f64::from(b) - weighted_mean[2]).powi(2) * weight;
    }

    let max_stddev = variance
        .into_iter()
        .map(|channel| (channel / weight_sum).sqrt())
        .fold(0.0, f64::max);
    if max_stddev > 30.0 {
        return None;
    }

    Some([
        weighted_mean[0].round() as u8,
        weighted_mean[1].round() as u8,
        weighted_mean[2].round() as u8,
    ])
}

pub fn build_monochrome_alpha_mask(
    image: &RgbaImage,
    color: [u8; 3],
    alpha_threshold: u8,
) -> RgbaImage {
    let mut output = RgbaImage::new(image.width(), image.height());
    for (target, source) in output.pixels_mut().zip(image.pixels()) {
        let alpha = if source[3] >= alpha_threshold { 255 } else { 0 };
        *target = Rgba([color[0], color[1], color[2], alpha]);
    }
    output
}

pub fn count_unique_colors(image: &RgbaImage) -> usize {
    image
        .pixels()
        .map(|pixel| pixel.0)
        .collect::<std::collections::HashSet<_>>()
        .len()
}

pub fn adaptive_trace_settings(analysis: &ImageAnalysis, quality: QualityPreset) -> TraceSettings {
    let mut settings = trace_settings_for_preset(quality);

    if should_trace_as_binary(analysis) {
        settings.color_mode = "binary";
        settings.layer_difference = 0;
        settings.filter_speckle = settings.filter_speckle.min(1);
    }

    settings
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

pub fn trace_settings_for_logo_preset(quality: LogoQualityPreset) -> TraceSettings {
    match quality {
        LogoQualityPreset::Clean => TraceSettings {
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
        LogoQualityPreset::Balanced => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 3,
            color_precision: 7,
            layer_difference: 12,
            length_threshold: 3.0,
            corner_threshold: 50,
            max_iterations: 12,
            splice_threshold: 40,
            path_precision: 6,
            optimizer_precision: 2,
        },
        LogoQualityPreset::High => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 2,
            color_precision: 8,
            layer_difference: 8,
            length_threshold: 2.0,
            corner_threshold: 40,
            max_iterations: 15,
            splice_threshold: 35,
            path_precision: 7,
            optimizer_precision: 2,
        },
        LogoQualityPreset::Ultra => TraceSettings {
            color_mode: "color",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 1,
            color_precision: 8,
            layer_difference: 4,
            length_threshold: 1.5,
            corner_threshold: 30,
            max_iterations: 20,
            splice_threshold: 30,
            path_precision: 8,
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

fn should_trace_as_binary(analysis: &ImageAnalysis) -> bool {
    matches!(analysis.image_type, ImageKind::Icon | ImageKind::Logo)
        && analysis.color_variance <= 8.0
        && analysis.unique_colors <= 48
        && analysis.top_10_coverage >= 0.95
}

#[cfg(test)]
mod tests {
    use image::RgbaImage;

    use super::{
        adaptive_trace_settings, build_monochrome_alpha_mask, detect_sparse_monochrome_color,
        trace_settings_for_logo_preset, trace_settings_for_preset, MONOCHROME_MASK_ALPHA_THRESHOLD,
    };
    use crate::types::{
        Complexity, ImageAnalysis, ImageKind, LogoQualityPreset, QualityPreset, TraceMode,
    };

    #[test]
    fn edgesvg_hifi_presets_match_reference_values() {
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

    #[test]
    fn edgesvg_logo_presets_match_reference_values() {
        let clean = trace_settings_for_logo_preset(LogoQualityPreset::Clean);
        assert_eq!(clean.filter_speckle, 4);
        assert_eq!(clean.corner_threshold, 60);

        let balanced = trace_settings_for_logo_preset(LogoQualityPreset::Balanced);
        assert_eq!(balanced.filter_speckle, 3);
        assert_eq!(balanced.layer_difference, 12);

        let high = trace_settings_for_logo_preset(LogoQualityPreset::High);
        assert_eq!(high.path_precision, 7);
        assert_eq!(high.length_threshold, 2.0);

        let ultra = trace_settings_for_logo_preset(LogoQualityPreset::Ultra);
        assert_eq!(ultra.filter_speckle, 1);
        assert_eq!(ultra.path_precision, 8);
    }

    #[test]
    fn monochrome_sparse_icons_switch_to_binary_trace_mode() {
        let analysis = ImageAnalysis {
            width: 256,
            height: 256,
            unique_colors: 24,
            top_10_coverage: 0.98,
            top_50_coverage: 1.0,
            color_variance: 0.0,
            edge_density: 0.08,
            alpha_coverage: 0.28,
            dominant_colors: vec!["#000000".to_string()],
            image_type: ImageKind::Icon,
            complexity: Complexity::Medium,
        };

        let settings = adaptive_trace_settings(&analysis, QualityPreset::Balanced);
        assert_eq!(settings.color_mode, "binary");
        assert_eq!(settings.layer_difference, 0);
    }

    #[test]
    fn detects_sparse_monochrome_color_from_transparent_icon() {
        let mut image = RgbaImage::new(8, 8);
        for y in 2..6 {
            for x in 2..6 {
                image.put_pixel(x, y, image::Rgba([12, 18, 24, 220]));
            }
        }

        let color = detect_sparse_monochrome_color(&image).expect("expected sparse monochrome");
        assert_eq!(color, [12, 18, 24]);
    }

    #[test]
    fn monochrome_alpha_mask_thresholds_transparency() {
        let mut image = RgbaImage::new(2, 1);
        image.put_pixel(0, 0, image::Rgba([10, 20, 30, 127]));
        image.put_pixel(1, 0, image::Rgba([10, 20, 30, 128]));

        let masked =
            build_monochrome_alpha_mask(&image, [10, 20, 30], MONOCHROME_MASK_ALPHA_THRESHOLD);

        assert_eq!(masked.get_pixel(0, 0).0, [10, 20, 30, 0]);
        assert_eq!(masked.get_pixel(1, 0).0, [10, 20, 30, 255]);
    }
}
