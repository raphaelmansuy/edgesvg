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

    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut sum = [0.0_f64; 3];
    let mut sum_sq = [0.0_f64; 3];

    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        let key = u32::from_be_bytes([r, g, b, a]);
        *counts.entry(key).or_default() += 1;
        for (channel, value) in [r, g, b].into_iter().enumerate() {
            let value = f64::from(value);
            sum[channel] += value;
            sum_sq[channel] += value * value;
        }
    }

    let unique_colors = counts.len();
    let mut ranked: Vec<(u32, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));

    let top_10_coverage =
        ranked.iter().take(10).map(|(_, c)| *c).sum::<usize>() as f64 / total_pixels as f64;
    let top_50_coverage =
        ranked.iter().take(50).map(|(_, c)| *c).sum::<usize>() as f64 / total_pixels as f64;

    let color_variance = (0..3)
        .map(|index| {
            let mean = sum[index] / total_pixels as f64;
            (sum_sq[index] / total_pixels as f64 - mean * mean)
                .max(0.0)
                .sqrt()
        })
        .sum::<f64>()
        / 3.0;

    let edge_density = estimate_edge_density(&DynamicImage::ImageRgba8(image.clone()).to_luma8());
    let dominant_colors = ranked
        .iter()
        .take(12)
        .map(|(color, _)| {
            let [r, g, b, _] = color.to_be_bytes();
            format!("#{r:02X}{g:02X}{b:02X}")
        })
        .collect::<Vec<_>>();

    let (image_type, complexity) = if unique_colors <= 12 && top_10_coverage > 0.95 {
        (ImageKind::Logo, Complexity::Simple)
    } else if unique_colors <= 32 && top_10_coverage > 0.90 {
        (ImageKind::Icon, Complexity::Medium)
    } else if color_variance < 40.0 && top_10_coverage > 0.85 {
        (ImageKind::Logo, Complexity::Simple)
    } else if color_variance < 60.0 && top_50_coverage > 0.90 {
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
        top_10_coverage,
        top_50_coverage,
        color_variance,
        edge_density,
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
}
