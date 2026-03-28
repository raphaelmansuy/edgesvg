use std::collections::HashMap;

use image::{DynamicImage, GrayImage, RgbaImage};

use crate::types::{Complexity, ImageAnalysis, ImageKind};

pub fn analyze_image(image: &DynamicImage) -> ImageAnalysis {
    let rgba = image.to_rgba8();
    analyze_rgba(&rgba)
}

pub fn analyze_rgba(image: &RgbaImage) -> ImageAnalysis {
    let (width, height) = image.dimensions();
    let total_pixels = (width as usize).saturating_mul(height as usize).max(1);
    let visible_alpha_threshold = 16u8;

    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut coarse_counts: HashMap<u16, usize> = HashMap::new();
    let mut sum = [0.0_f64; 3];
    let mut sum_sq = [0.0_f64; 3];
    let mut visible_pixels = 0usize;

    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < visible_alpha_threshold {
            continue;
        }

        visible_pixels += 1;
        let key = u32::from_be_bytes([r, g, b, a]);
        *counts.entry(key).or_default() += 1;
        let coarse_key =
            (u16::from(r >> 4) << 8) | (u16::from(g >> 4) << 4) | u16::from(b >> 4);
        *coarse_counts.entry(coarse_key).or_default() += 1;
        for (channel, value) in [r, g, b].into_iter().enumerate() {
            let value = f64::from(value);
            sum[channel] += value;
            sum_sq[channel] += value * value;
        }
    }

    let measured_pixels = visible_pixels.max(1);
    let alpha_coverage = visible_pixels as f64 / total_pixels as f64;
    let unique_colors = counts.len();
    let effective_colors = coarse_counts.len();
    let mut ranked: Vec<(u32, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    let mut coarse_ranked: Vec<(u16, usize)> = coarse_counts.into_iter().collect();
    coarse_ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let top_10_coverage =
        ranked.iter().take(10).map(|(_, c)| *c).sum::<usize>() as f64 / measured_pixels as f64;
    let top_50_coverage =
        ranked.iter().take(50).map(|(_, c)| *c).sum::<usize>() as f64 / measured_pixels as f64;
    let coarse_top_12_coverage = coarse_ranked.iter().take(12).map(|(_, c)| *c).sum::<usize>()
        as f64
        / measured_pixels as f64;

    let color_variance = (0..3)
        .map(|index| {
            let mean = sum[index] / measured_pixels as f64;
            (sum_sq[index] / measured_pixels as f64 - mean * mean)
                .max(0.0)
                .sqrt()
        })
        .sum::<f64>()
        / 3.0;

    let edge_density = if alpha_coverage < 0.95 {
        estimate_alpha_edge_density(image)
    } else {
        estimate_edge_density(&DynamicImage::ImageRgba8(image.clone()).to_luma8())
    };
    let dominant_colors = ranked
        .iter()
        .take(12)
        .map(|(color, _)| {
            let [r, g, b, _] = color.to_be_bytes();
            format!("#{r:02X}{g:02X}{b:02X}")
        })
        .collect::<Vec<_>>();

    let sparse_transparent_graphic = alpha_coverage < 0.60 && coarse_top_12_coverage > 0.90;
    let antialiased_flat_graphic = alpha_coverage < 0.80
        && coarse_top_12_coverage > 0.94
        && effective_colors <= 48
        && edge_density < 0.04;
    let mostly_monochrome = effective_colors <= 12 && coarse_top_12_coverage > 0.92;

    let (image_type, complexity) = if sparse_transparent_graphic && mostly_monochrome {
        if effective_colors <= 6 {
            (ImageKind::Logo, Complexity::Simple)
        } else {
            (ImageKind::Icon, Complexity::Medium)
        }
    } else if antialiased_flat_graphic {
        if effective_colors <= 8 && color_variance < 55.0 {
            (ImageKind::Logo, Complexity::Simple)
        } else {
            (ImageKind::Icon, Complexity::Medium)
        }
    } else if unique_colors <= 12 && top_10_coverage > 0.95 {
        (ImageKind::Logo, Complexity::Simple)
    } else if effective_colors <= 32 && coarse_top_12_coverage > 0.92 {
        (ImageKind::Icon, Complexity::Medium)
    } else if color_variance < 40.0 && top_10_coverage > 0.85 && effective_colors <= 24 {
        (ImageKind::Logo, Complexity::Simple)
    } else if color_variance < 60.0 && top_50_coverage > 0.90 && effective_colors <= 48 {
        (ImageKind::Icon, Complexity::Medium)
    } else if color_variance < 80.0 && edge_density < 0.15 {
        (ImageKind::Illustration, Complexity::Medium)
    } else {
        (ImageKind::Photo, Complexity::Complex)
    };

    ImageAnalysis {
        width,
        height,
        unique_colors,
        effective_colors,
        top_10_coverage,
        top_50_coverage,
        color_variance,
        edge_density,
        alpha_coverage,
        dominant_colors,
        image_type,
        complexity,
    }
}

fn estimate_edge_density(gray: &GrayImage) -> f64 {
    let (width, height) = gray.dimensions();
    if width < 3 || height < 3 {
        return 0.0;
    }

    let mut edges = 0usize;
    let mut samples = 0usize;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let p = |dx: i32, dy: i32| -> f64 {
                f64::from(gray.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[0])
            };

            let gx = -p(-1, -1) + p(1, -1) - 2.0 * p(-1, 0) + 2.0 * p(1, 0) - p(-1, 1) + p(1, 1);
            let gy = -p(-1, -1) - 2.0 * p(0, -1) - p(1, -1) + p(-1, 1) + 2.0 * p(0, 1) + p(1, 1);
            let magnitude = (gx * gx + gy * gy).sqrt();
            if magnitude > 120.0 {
                edges += 1;
            }
            samples += 1;
        }
    }

    if samples == 0 {
        0.0
    } else {
        edges as f64 / samples as f64
    }
}

fn estimate_alpha_edge_density(image: &RgbaImage) -> f64 {
    let (width, height) = image.dimensions();
    if width < 3 || height < 3 {
        return 0.0;
    }

    let mut edges = 0usize;
    let mut samples = 0usize;

    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let p = |dx: i32, dy: i32| -> f64 {
                f64::from(image.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[3])
            };

            let gx = -p(-1, -1) + p(1, -1) - 2.0 * p(-1, 0) + 2.0 * p(1, 0) - p(-1, 1) + p(1, 1);
            let gy = -p(-1, -1) - 2.0 * p(0, -1) - p(1, -1) + p(-1, 1) + 2.0 * p(0, 1) + p(1, 1);
            let magnitude = (gx * gx + gy * gy).sqrt();
            if magnitude > 64.0 {
                edges += 1;
            }
            samples += 1;
        }
    }

    if samples == 0 {
        0.0
    } else {
        edges as f64 / samples as f64
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, Rgba, RgbaImage};

    use super::analyze_image;
    use crate::types::ImageKind;

    #[test]
    fn classifies_simple_three_color_input_as_logo() {
        let mut image = RgbaImage::new(64, 32);
        for y in 0..32 {
            for x in 0..64 {
                let pixel = if x < 21 {
                    Rgba([255, 255, 255, 255])
                } else if x < 42 {
                    Rgba([180, 0, 0, 255])
                } else {
                    Rgba([0, 0, 180, 255])
                };
                image.put_pixel(x, y, pixel);
            }
        }

        let analysis = analyze_image(&DynamicImage::ImageRgba8(image));
        assert_eq!(analysis.image_type, ImageKind::Logo);
        assert_eq!(analysis.width, 64);
        assert_eq!(analysis.height, 32);
        assert!(analysis.top_10_coverage > 0.99);
    }

    #[test]
    fn classifies_transparent_antialiased_multicolor_graphic_as_icon() {
        let mut image = RgbaImage::new(64, 64);
        for y in 8..56 {
            for x in 8..56 {
                let alpha = if x == 8 || y == 8 || x == 55 || y == 55 {
                    160
                } else {
                    255
                };
                let pixel = match (x < 32, y < 32) {
                    (true, true) => Rgba([66, 133, 244, alpha]),
                    (false, true) => Rgba([234, 67, 53, alpha]),
                    (true, false) => Rgba([251, 188, 5, alpha]),
                    (false, false) => Rgba([52, 168, 83, alpha]),
                };
                image.put_pixel(x, y, pixel);
            }
        }

        let analysis = analyze_image(&DynamicImage::ImageRgba8(image));
        assert!(matches!(analysis.image_type, ImageKind::Icon | ImageKind::Logo));
        assert!(analysis.effective_colors <= 16);
    }
}
