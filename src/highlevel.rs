use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, RgbaImage};
use regex::Regex;
use serde::Serialize;

use crate::analysis::analyze_image;
use crate::metrics::{compute_metrics, compute_metrics_against, PreparedMetricsInput};
use crate::preprocess::{
    build_monochrome_alpha_mask, preprocess_image, quantize_image, trace_settings_for_logo_preset,
    trace_settings_for_preset, MONOCHROME_MASK_ALPHA_THRESHOLD,
};
use crate::svg::optimize_svg;
use crate::types::{AutoMode, ImageKind, LogoQualityPreset, QualityPreset, VectorizationReport};
use crate::vectorizer::trace_to_svg;

pub type RgbColor = (u8, u8, u8);
pub type MonochromeIconDetection = (bool, Option<RgbColor>);

#[derive(Debug, Clone, Serialize)]
pub struct AutoDecision {
    pub mode: AutoMode,
    pub logo_quality: LogoQualityPreset,
    pub monochrome_color: Option<RgbColor>,
}

pub fn is_monochrome_icon(input_path: &Path) -> Result<MonochromeIconDetection> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?
        .to_rgba8();
    Ok(is_monochrome_icon_rgba(&image))
}

fn is_monochrome_icon_rgba(image: &RgbaImage) -> MonochromeIconDetection {
    let mut visible = Vec::<[u8; 3]>::new();
    let mut opaque = 0usize;
    for pixel in image.pixels() {
        if pixel[3] > 10 {
            opaque += 1;
            visible.push([pixel[0], pixel[1], pixel[2]]);
        }
    }
    if visible.is_empty() {
        return (false, None);
    }

    let opaque_ratio = opaque as f64 / (image.width() as f64 * image.height() as f64);
    if opaque_ratio > 0.95 {
        return (false, None);
    }

    let avg = average_rgb(&visible);
    let std = channel_stddev(&visible, avg);
    if std.0.max(std.1).max(std.2) > 30.0 {
        return (false, None);
    }

    (
        true,
        Some((
            avg.0.round() as u8,
            avg.1.round() as u8,
            avg.2.round() as u8,
        )),
    )
}

pub fn determine_auto_mode(input_path: &Path) -> Result<AutoDecision> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    Ok(determine_auto_mode_image(&image))
}

pub fn determine_auto_mode_image(image: &DynamicImage) -> AutoDecision {
    let (mono, color) = is_monochrome_icon_rgba(&image.to_rgba8());
    if mono {
        return AutoDecision {
            mode: AutoMode::Logo,
            logo_quality: LogoQualityPreset::Ultra,
            monochrome_color: color,
        };
    }

    let analysis = analyze_image(image);
    if matches!(analysis.image_type, ImageKind::Logo | ImageKind::Icon)
        && analysis.unique_colors <= 1000
    {
        AutoDecision {
            mode: AutoMode::Logo,
            logo_quality: if analysis.top_10_coverage > 0.90 {
                LogoQualityPreset::Clean
            } else {
                LogoQualityPreset::Ultra
            },
            monochrome_color: None,
        }
    } else {
        AutoDecision {
            mode: AutoMode::Premium,
            logo_quality: LogoQualityPreset::Ultra,
            monochrome_color: None,
        }
    }
}

pub fn vectorize_auto(input_path: &Path) -> Result<(String, VectorizationReport)> {
    let image = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_auto_image(&image)
}

pub fn vectorize_auto_image(original: &DynamicImage) -> Result<(String, VectorizationReport)> {
    let mut outcomes = Vec::new();

    if let Ok((svg, report)) =
        vectorize_logo_premium_image(original, LogoQualityPreset::Ultra, None)
    {
        outcomes.push(("logo_ultra", svg, report));
    }
    if let Ok((svg, report)) =
        vectorize_logo_premium_image(original, LogoQualityPreset::Clean, None)
    {
        outcomes.push(("logo_clean", svg, report));
    }
    if let Ok((svg, report)) = vectorize_premium_image(original, 0.95, Some(32)) {
        outcomes.push(("premium_photo", svg, report));
    }
    if let Ok((svg, report)) = vectorize_smart_image(original, 0.95, 100_000, 4) {
        outcomes.push(("smart", svg, report));
    }

    let winner = outcomes
        .into_iter()
        .max_by(|left, right| {
            auto_score(&left.2)
                .partial_cmp(&auto_score(&right.2))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .context("auto vectorization produced no successful candidate")?;

    Ok((winner.1, winner.2))
}

pub fn vectorize_logo_premium(
    input_path: &Path,
    quality: LogoQualityPreset,
    n_colors: Option<usize>,
) -> Result<(String, VectorizationReport)> {
    let original = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_logo_premium_image(&original, quality, n_colors)
}

pub fn vectorize_logo_premium_image(
    original: &DynamicImage,
    quality: LogoQualityPreset,
    n_colors: Option<usize>,
) -> Result<(String, VectorizationReport)> {
    let (mono, color) = is_monochrome_icon_rgba(&original.to_rgba8());
    if mono {
        if let Some(color) = color {
            return process_geometric_icon(original, color, quality);
        }
    }

    let analysis = analyze_image(original);
    let palette_override = n_colors.or(match analysis.image_type {
        ImageKind::Logo => Some(8),
        ImageKind::Icon => Some(12),
        _ => None,
    });
    let mut candidate = preprocess_image(original, &analysis, palette_override)?;
    candidate = DynamicImage::ImageRgba8(candidate)
        .unsharpen(1.0, 1)
        .to_rgba8();

    let settings = trace_settings_for_logo_preset(quality);
    let svg = trace_to_svg(&candidate, &settings)?;
    let svg = snap_svg_colors(&svg);
    let svg = optimize_svg(&svg, settings.optimizer_precision);
    let metrics = compute_metrics(original, &svg)?;
    Ok((
        svg,
        VectorizationReport {
            analysis,
            settings,
            quality_preset: map_logo_quality(quality),
            metrics,
        },
    ))
}

pub fn vectorize_premium(
    input_path: &Path,
    target_ssim: f64,
    n_colors: Option<usize>,
) -> Result<(String, VectorizationReport)> {
    let original = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_premium_image(&original, target_ssim, n_colors)
}

pub fn vectorize_premium_image(
    original: &DynamicImage,
    target_ssim: f64,
    n_colors: Option<usize>,
) -> Result<(String, VectorizationReport)> {
    let analysis = analyze_image(original);
    if matches!(analysis.image_type, ImageKind::Logo | ImageKind::Icon)
        && analysis.unique_colors <= 256
    {
        let quality = if target_ssim >= 0.98 {
            LogoQualityPreset::Clean
        } else {
            LogoQualityPreset::Balanced
        };
        return vectorize_logo_premium_image(original, quality, n_colors);
    }
    let mut candidate = prepare_premium_candidate(original, &analysis, n_colors, target_ssim);

    let ladder = if target_ssim >= 0.98 {
        vec![QualityPreset::Quality, QualityPreset::Ultra]
    } else {
        vec![QualityPreset::Balanced, QualityPreset::Quality]
    };

    let mut best = None;
    let prepared_metrics = PreparedMetricsInput::from_image(original);
    for quality in ladder {
        let settings = trace_settings_for_preset(quality);
        let svg = trace_to_svg(&candidate, &settings)?;
        let svg = snap_svg_colors(&svg);
        let svg = optimize_svg(&svg, settings.optimizer_precision);
        let metrics = compute_metrics_against(&prepared_metrics, &svg)?;
        let score =
            metrics.ssim * 0.6 + metrics.ssim_perceptual * 0.2 + metrics.edge_similarity * 0.2;
        let report = VectorizationReport {
            analysis: analysis.clone(),
            settings,
            quality_preset: quality,
            metrics: metrics.clone(),
        };
        if best
            .as_ref()
            .map(|(_, _, best_score): &(String, VectorizationReport, f64)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((svg, report, score));
        }

        if metrics.ssim >= target_ssim {
            break;
        }
        let reduced_colors = count_unique_colors_rgba(&candidate).saturating_mul(3) / 4;
        if reduced_colors >= 4 {
            candidate = quantize_image(&candidate, reduced_colors);
        }
    }

    let (svg, report, _) = best.context("premium vectorization produced no candidate")?;
    Ok((svg, report))
}

pub fn vectorize_optimal(input_path: &Path) -> Result<(String, VectorizationReport)> {
    let original = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_optimal_image(&original)
}

pub fn vectorize_optimal_image(original: &DynamicImage) -> Result<(String, VectorizationReport)> {
    let analysis = analyze_image(original);
    let mut candidate = preprocess_image(original, &analysis, None)?;
    candidate = DynamicImage::ImageRgba8(candidate).blur(0.4).to_rgba8();
    candidate = DynamicImage::ImageRgba8(candidate)
        .unsharpen(1.0, 1)
        .to_rgba8();

    let settings = trace_settings_for_preset(QualityPreset::Quality);
    let svg = trace_to_svg(&candidate, &settings)?;
    let svg = snap_svg_colors(&svg);
    let svg = optimize_svg(&svg, settings.optimizer_precision);
    let metrics = compute_metrics(original, &svg)?;

    Ok((
        svg,
        VectorizationReport {
            analysis,
            settings,
            quality_preset: QualityPreset::Quality,
            metrics,
        },
    ))
}

pub fn vectorize_smart(
    input_path: &Path,
    target_ssim: f64,
    max_file_size: usize,
    max_iterations: usize,
) -> Result<(String, VectorizationReport)> {
    let original = image::open(input_path)
        .with_context(|| format!("unable to open image {}", input_path.display()))?;
    vectorize_smart_image(&original, target_ssim, max_file_size, max_iterations)
}

pub fn vectorize_smart_image(
    original: &DynamicImage,
    target_ssim: f64,
    max_file_size: usize,
    max_iterations: usize,
) -> Result<(String, VectorizationReport)> {
    let analysis = analyze_image(original);
    let mut candidate = preprocess_image(original, &analysis, None)?;
    let ladder = [
        QualityPreset::Figma,
        QualityPreset::Balanced,
        QualityPreset::Quality,
    ];

    let mut best: Option<(String, VectorizationReport, f64)> = None;
    let prepared_metrics = PreparedMetricsInput::from_image(original);
    for iteration in 0..max_iterations.max(1) {
        let quality = ladder[iteration.min(ladder.len() - 1)];
        let settings = trace_settings_for_preset(quality);
        let mut svg = trace_to_svg(&candidate, &settings)?;
        if !matches!(analysis.image_type, ImageKind::Photo) {
            svg = snap_svg_colors(&svg);
        }
        let svg = optimize_svg(&svg, settings.optimizer_precision);
        let metrics = compute_metrics_against(&prepared_metrics, &svg)?;
        let report = VectorizationReport {
            analysis: analysis.clone(),
            settings,
            quality_preset: quality,
            metrics: metrics.clone(),
        };
        let score = smart_score(&report, max_file_size);

        if best
            .as_ref()
            .map(|(_, _, best_score)| score > *best_score)
            .unwrap_or(true)
        {
            best = Some((svg.clone(), report.clone(), score));
        }

        if metrics.ssim >= target_ssim && metrics.file_size <= max_file_size {
            return Ok((svg, report));
        }

        if metrics.file_size > max_file_size {
            let reduced_colors = ((count_unique_colors_rgba(&candidate) as f64) * 0.7) as usize;
            candidate = quantize_image(&candidate, reduced_colors.clamp(4, 256));
        }
    }

    let (svg, report, _) = best.context("smart vectorization produced no candidate")?;
    Ok((svg, report))
}

fn process_geometric_icon(
    original: &DynamicImage,
    color: RgbColor,
    quality: LogoQualityPreset,
) -> Result<(String, VectorizationReport)> {
    let mask = build_monochrome_alpha_mask(
        &original.to_rgba8(),
        [color.0, color.1, color.2],
        MONOCHROME_MASK_ALPHA_THRESHOLD,
    );
    let analysis = analyze_image(&DynamicImage::ImageRgba8(mask.clone()));
    let settings = trace_settings_for_logo_preset(quality);
    let svg = trace_to_svg(&mask, &settings)?;
    let svg = optimize_svg(&svg, settings.optimizer_precision);
    let metrics = compute_metrics(original, &svg)?;
    Ok((
        svg,
        VectorizationReport {
            analysis,
            settings,
            quality_preset: map_logo_quality(quality),
            metrics,
        },
    ))
}

fn prepare_premium_candidate(
    original: &DynamicImage,
    analysis: &crate::types::ImageAnalysis,
    n_colors: Option<usize>,
    target_ssim: f64,
) -> RgbaImage {
    let mut candidate = original.to_rgba8();
    let palette_size = n_colors.unwrap_or_else(|| {
        if analysis.unique_colors <= 16 {
            analysis.unique_colors.max(4)
        } else if target_ssim >= 0.98 {
            24
        } else if matches!(analysis.image_type, ImageKind::Photo) {
            32
        } else {
            16
        }
    });
    candidate = quantize_image(&candidate, palette_size.clamp(4, 64));
    candidate = DynamicImage::ImageRgba8(candidate).blur(0.3).to_rgba8();
    DynamicImage::ImageRgba8(candidate)
        .unsharpen(1.5, 1)
        .to_rgba8()
}

fn map_logo_quality(quality: LogoQualityPreset) -> QualityPreset {
    match quality {
        LogoQualityPreset::Clean => QualityPreset::Figma,
        LogoQualityPreset::Balanced => QualityPreset::Balanced,
        LogoQualityPreset::High => QualityPreset::Quality,
        LogoQualityPreset::Ultra => QualityPreset::Ultra,
    }
}

fn snap_svg_colors(svg: &str) -> String {
    let fill_re = Regex::new(r#"fill="([^"]+)""#).expect("valid fill regex");
    fill_re
        .replace_all(svg, |caps: &regex::Captures<'_>| {
            let value = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            format!(r#"fill="{}""#, snap_fill_color(value))
        })
        .into_owned()
}

fn snap_fill_color(color: &str) -> String {
    let normalized = color.trim().to_ascii_lowercase();
    let rgb = match parse_hex_color(&normalized) {
        Some(rgb) => rgb,
        None => return normalized,
    };
    let snapped = snap_to_standard_color(rgb);
    format!("#{:02x}{:02x}{:02x}", snapped.0, snapped.1, snapped.2)
}

fn parse_hex_color(color: &str) -> Option<(u8, u8, u8)> {
    let value = color.strip_prefix('#')?;
    if value.len() == 3 {
        let expand = |c: char| -> Option<u8> {
            let s = [c, c].iter().collect::<String>();
            u8::from_str_radix(&s, 16).ok()
        };
        let mut chars = value.chars();
        Some((
            expand(chars.next()?)?,
            expand(chars.next()?)?,
            expand(chars.next()?)?,
        ))
    } else if value.len() == 6 {
        Some((
            u8::from_str_radix(&value[0..2], 16).ok()?,
            u8::from_str_radix(&value[2..4], 16).ok()?,
            u8::from_str_radix(&value[4..6], 16).ok()?,
        ))
    } else {
        None
    }
}

fn snap_to_standard_color(color: (u8, u8, u8)) -> (u8, u8, u8) {
    const STANDARD_COLORS: &[(u8, u8, u8)] = &[
        (0, 0, 0),
        (255, 255, 255),
        (255, 0, 0),
        (0, 255, 0),
        (0, 0, 255),
        (255, 255, 0),
        (0, 255, 255),
        (255, 0, 255),
    ];
    STANDARD_COLORS
        .iter()
        .copied()
        .find(|candidate| color_distance(color, *candidate) < 15.0)
        .unwrap_or(color)
}

fn color_distance(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let dr = f64::from(a.0) - f64::from(b.0);
    let dg = f64::from(a.1) - f64::from(b.1);
    let db = f64::from(a.2) - f64::from(b.2);
    (dr * dr + dg * dg + db * db).sqrt()
}

fn smart_score(report: &VectorizationReport, max_file_size: usize) -> f64 {
    let file_size = report.metrics.file_size as f64;
    let size_score = if file_size < (max_file_size as f64 * 3.0) {
        (1.0 - file_size / max_file_size.max(1) as f64).max(0.0)
    } else {
        -1.0
    };
    report.metrics.ssim * 0.7 + size_score * 0.3
}

fn auto_score(report: &VectorizationReport) -> f64 {
    report.metrics.ssim * 100.0 - (report.metrics.file_size as f64 / 1024.0 / 100.0)
}

fn average_rgb(colors: &[[u8; 3]]) -> (f64, f64, f64) {
    let len = colors.len().max(1) as f64;
    let mut sum = (0.0, 0.0, 0.0);
    for color in colors {
        sum.0 += f64::from(color[0]);
        sum.1 += f64::from(color[1]);
        sum.2 += f64::from(color[2]);
    }
    (sum.0 / len, sum.1 / len, sum.2 / len)
}

fn channel_stddev(colors: &[[u8; 3]], mean: (f64, f64, f64)) -> (f64, f64, f64) {
    let len = colors.len().max(1) as f64;
    let mut sum = (0.0, 0.0, 0.0);
    for color in colors {
        sum.0 += (f64::from(color[0]) - mean.0).powi(2);
        sum.1 += (f64::from(color[1]) - mean.1).powi(2);
        sum.2 += (f64::from(color[2]) - mean.2).powi(2);
    }
    (
        (sum.0 / len).sqrt(),
        (sum.1 / len).sqrt(),
        (sum.2 / len).sqrt(),
    )
}

fn count_unique_colors_rgba(image: &RgbaImage) -> usize {
    let mut set = HashSet::new();
    for pixel in image.pixels() {
        set.insert(pixel.0);
    }
    set.len()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use image::{Rgba, RgbaImage};
    use tempfile::tempdir;

    use super::{
        determine_auto_mode, is_monochrome_icon, vectorize_auto, vectorize_logo_premium,
        vectorize_optimal, vectorize_premium, vectorize_smart,
    };
    use crate::types::{AutoMode, LogoQualityPreset};

    #[test]
    fn detects_transparent_monochrome_icon() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("icon.png");
        let mut image = RgbaImage::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                let pixel = if x > 3 && x < 12 && y > 3 && y < 12 {
                    Rgba([20, 30, 40, 255])
                } else {
                    Rgba([0, 0, 0, 0])
                };
                image.put_pixel(x, y, pixel);
            }
        }
        image.save(&path).unwrap();
        let (is_icon, color) = is_monochrome_icon(&path).unwrap();
        assert!(is_icon);
        assert!(color.is_some());
    }

    #[test]
    fn auto_mode_prefers_logo_for_simple_art() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("logo.png");
        let mut image = RgbaImage::new(24, 24);
        for y in 0..24 {
            for x in 0..24 {
                let pixel = if x < 12 {
                    Rgba([255, 0, 0, 255])
                } else {
                    Rgba([255, 255, 255, 255])
                };
                image.put_pixel(x, y, pixel);
            }
        }
        image.save(&path).unwrap();
        let decision = determine_auto_mode(&path).unwrap();
        assert_eq!(decision.mode, AutoMode::Logo);
    }

    #[test]
    fn logo_and_premium_vectorization_paths_work() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("input.png");
        let mut image = RgbaImage::new(32, 32);
        for y in 0..32 {
            for x in 0..32 {
                let pixel = if x < 16 {
                    Rgba([255, 0, 0, 255])
                } else {
                    Rgba([0, 0, 255, 255])
                };
                image.put_pixel(x, y, pixel);
            }
        }
        image.save(&path).unwrap();

        let (logo_svg, _) =
            vectorize_logo_premium(&path, LogoQualityPreset::Clean, Some(2)).unwrap();
        assert!(logo_svg.contains("<svg"));

        let (premium_svg, _) = vectorize_premium(&path, 0.9, Some(2)).unwrap();
        assert!(premium_svg.contains("<svg"));

        let (smart_svg, _) = vectorize_smart(&path, 0.9, 100_000, 3).unwrap();
        assert!(smart_svg.contains("<svg"));

        let (optimal_svg, _) = vectorize_optimal(&path).unwrap();
        assert!(optimal_svg.contains("<svg"));

        let (auto_svg, _) = vectorize_auto(&path).unwrap();
        assert!(auto_svg.contains("<svg"));

        fs::remove_file(path).ok();
    }
}
