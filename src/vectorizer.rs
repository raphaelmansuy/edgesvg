use std::fmt;

use anyhow::{anyhow, Result};
use fastrand::Rng;
use image::RgbaImage;
use visioncortex::color_clusters::{KeyingAction, Runner, RunnerConfig, HIERARCHICAL_MAX};
use visioncortex::{Color, ColorImage, ColorName, CompoundPath, PointF64};

use crate::types::{TraceMode, TraceSettings};

const NUM_UNUSED_COLOR_ITERATIONS: usize = 6;
const KEYING_THRESHOLD: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorMode {
    Color,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HierarchicalMode {
    Stacked,
    Cutout,
}

#[derive(Debug, Clone)]
pub(crate) struct ConversionConfig {
    pub color_mode: ColorMode,
    pub hierarchical: HierarchicalMode,
    pub filter_speckle: usize,
    pub color_precision: i32,
    pub layer_difference: i32,
    pub mode: TraceMode,
    pub corner_threshold: i32,
    pub length_threshold: f64,
    pub max_iterations: usize,
    pub splice_threshold: i32,
    pub path_precision: Option<u32>,
}

impl ConversionConfig {
    pub(crate) fn from_trace_settings(settings: &TraceSettings) -> Self {
        Self {
            color_mode: match settings.color_mode {
                "binary" => ColorMode::Binary,
                _ => ColorMode::Color,
            },
            hierarchical: match settings.hierarchical {
                "cutout" => HierarchicalMode::Cutout,
                _ => HierarchicalMode::Stacked,
            },
            filter_speckle: settings.filter_speckle,
            color_precision: settings.color_precision,
            layer_difference: settings.layer_difference,
            mode: settings.mode,
            corner_threshold: settings.corner_threshold,
            length_threshold: settings.length_threshold,
            max_iterations: settings.max_iterations,
            splice_threshold: settings.splice_threshold,
            path_precision: Some(settings.path_precision),
        }
    }

    fn filter_speckle_area(&self) -> usize {
        self.filter_speckle * self.filter_speckle
    }

    fn color_precision_loss(&self) -> i32 {
        8 - self.color_precision
    }

    fn corner_threshold_radians(&self) -> f64 {
        self.corner_threshold as f64 / 180.0 * std::f64::consts::PI
    }

    fn splice_threshold_radians(&self) -> f64 {
        self.splice_threshold as f64 / 180.0 * std::f64::consts::PI
    }
}

#[derive(Debug, Clone)]
struct SvgFile {
    paths: Vec<SvgPath>,
    width: usize,
    height: usize,
    path_precision: Option<u32>,
}

#[derive(Debug, Clone)]
struct SvgPath {
    path: CompoundPath,
    color: Color,
}

impl SvgFile {
    fn new(width: usize, height: usize, path_precision: Option<u32>) -> Self {
        Self {
            paths: Vec::new(),
            width,
            height,
            path_precision,
        }
    }

    fn add_path(&mut self, path: CompoundPath, color: Color) {
        self.paths.push(SvgPath { path, color });
    }
}

impl fmt::Display for SvgFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(f, r#"<!-- Generator: vectalab internal vectorizer -->"#)?;
        writeln!(
            f,
            r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">"#,
            self.width, self.height
        )?;

        for path in &self.paths {
            path.fmt_with_precision(f, self.path_precision)?;
        }

        writeln!(f, "</svg>")
    }
}

impl SvgPath {
    fn fmt_with_precision(
        &self,
        f: &mut fmt::Formatter<'_>,
        precision: Option<u32>,
    ) -> fmt::Result {
        let (string, offset) = self
            .path
            .to_svg_string(true, PointF64::default(), precision);
        writeln!(
            f,
            r#"<path d="{}" fill="{}" transform="translate({},{})"/>"#,
            string,
            self.color.to_hex_string(),
            offset.x,
            offset.y
        )
    }
}

pub(crate) fn trace_to_svg(image: &RgbaImage, settings: &TraceSettings) -> Result<String> {
    let config = ConversionConfig::from_trace_settings(settings);
    let color_image = ColorImage {
        pixels: image.clone().into_raw(),
        width: image.width() as usize,
        height: image.height() as usize,
    };
    let svg = convert(color_image, config).map_err(|error| anyhow!(error))?;
    Ok(svg.to_string())
}

fn convert(img: ColorImage, config: ConversionConfig) -> Result<SvgFile, String> {
    match config.color_mode {
        ColorMode::Color => color_image_to_svg(img, config),
        ColorMode::Binary => binary_image_to_svg(img, config),
    }
}

fn color_exists_in_image(img: &ColorImage, color: Color) -> bool {
    for y in 0..img.height {
        for x in 0..img.width {
            let pixel_color = img.get_pixel(x, y);
            if pixel_color.r == color.r && pixel_color.g == color.g && pixel_color.b == color.b {
                return true;
            }
        }
    }
    false
}

fn find_unused_color_in_image(img: &ColorImage) -> Result<Color, String> {
    let special_colors = [
        Color::new(255, 0, 0),
        Color::new(0, 255, 0),
        Color::new(0, 0, 255),
        Color::new(255, 255, 0),
        Color::new(0, 255, 255),
        Color::new(255, 0, 255),
    ];
    let mut rng = Rng::new();
    let random_colors =
        (0..NUM_UNUSED_COLOR_ITERATIONS).map(|_| Color::new(rng.u8(..), rng.u8(..), rng.u8(..)));

    for color in special_colors.into_iter().chain(random_colors) {
        if !color_exists_in_image(img, color) {
            return Ok(color);
        }
    }

    Err("unable to find unused color in image to use as key".to_string())
}

fn should_key_image(img: &ColorImage) -> bool {
    if img.width == 0 || img.height == 0 {
        return false;
    }

    let threshold = ((img.width * 2) as f32 * KEYING_THRESHOLD) as usize;
    let mut transparent = 0usize;
    let y_positions = [
        0,
        img.height / 4,
        img.height / 2,
        3 * img.height / 4,
        img.height - 1,
    ];

    for y in y_positions {
        for x in 0..img.width {
            if img.get_pixel(x, y).a == 0 {
                transparent += 1;
            }
            if transparent >= threshold {
                return true;
            }
        }
    }

    false
}

fn color_image_to_svg(mut img: ColorImage, config: ConversionConfig) -> Result<SvgFile, String> {
    let width = img.width;
    let height = img.height;

    let key_color = if should_key_image(&img) {
        let key_color = find_unused_color_in_image(&img)?;
        for y in 0..height {
            for x in 0..width {
                if img.get_pixel(x, y).a == 0 {
                    img.set_pixel(x, y, &key_color);
                }
            }
        }
        key_color
    } else {
        Color::default()
    };

    let runner = Runner::new(
        RunnerConfig {
            diagonal: config.layer_difference == 0,
            hierarchical: HIERARCHICAL_MAX,
            batch_size: 25_600,
            good_min_area: config.filter_speckle_area(),
            good_max_area: width * height,
            is_same_color_a: config.color_precision_loss(),
            is_same_color_b: 1,
            deepen_diff: config.layer_difference,
            hollow_neighbours: 1,
            key_color,
            keying_action: if matches!(config.hierarchical, HierarchicalMode::Cutout) {
                KeyingAction::Keep
            } else {
                KeyingAction::Discard
            },
        },
        img,
    );

    let mut clusters = runner.run();

    if matches!(config.hierarchical, HierarchicalMode::Cutout) {
        let view = clusters.view();
        let image = view.to_color_image();
        let runner = Runner::new(
            RunnerConfig {
                diagonal: false,
                hierarchical: 64,
                batch_size: 25_600,
                good_min_area: 0,
                good_max_area: image.width * image.height,
                is_same_color_a: 0,
                is_same_color_b: 1,
                deepen_diff: 0,
                hollow_neighbours: 0,
                key_color,
                keying_action: KeyingAction::Discard,
            },
            image,
        );
        clusters = runner.run();
    }

    let view = clusters.view();
    let mut svg = SvgFile::new(width, height, config.path_precision);

    for &cluster_index in view.clusters_output.iter().rev() {
        let cluster = view.get_cluster(cluster_index);
        let paths = cluster.to_compound_path(
            &view,
            false,
            match config.mode {
                TraceMode::Spline => visioncortex::PathSimplifyMode::Spline,
                TraceMode::Polygon => visioncortex::PathSimplifyMode::Polygon,
            },
            config.corner_threshold_radians(),
            config.length_threshold,
            config.max_iterations,
            config.splice_threshold_radians(),
        );
        svg.add_path(paths, cluster.residue_color());
    }

    Ok(svg)
}

fn binary_image_to_svg(img: ColorImage, config: ConversionConfig) -> Result<SvgFile, String> {
    let img = img.to_binary_image(|x| x.r < 128);
    let width = img.width;
    let height = img.height;
    let clusters = img.to_clusters(false);

    let mut svg = SvgFile::new(width, height, config.path_precision);
    for index in 0..clusters.len() {
        let cluster = clusters.get_cluster(index);
        if cluster.size() >= config.filter_speckle_area() {
            let paths = cluster.to_compound_path(
                match config.mode {
                    TraceMode::Spline => visioncortex::PathSimplifyMode::Spline,
                    TraceMode::Polygon => visioncortex::PathSimplifyMode::Polygon,
                },
                config.corner_threshold_radians(),
                config.length_threshold,
                config.max_iterations,
                config.splice_threshold_radians(),
            );
            svg.add_path(paths, Color::color(&ColorName::Black));
        }
    }

    Ok(svg)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::trace_to_svg;
    use crate::preprocess::trace_settings_for_preset;
    use crate::types::QualityPreset;

    #[test]
    fn internal_vectorizer_traces_simple_color_regions() {
        let mut image = RgbaImage::new(8, 4);
        for y in 0..4 {
            for x in 0..8 {
                let pixel = if x < 4 {
                    Rgba([255, 0, 0, 255])
                } else {
                    Rgba([0, 0, 255, 255])
                };
                image.put_pixel(x, y, pixel);
            }
        }

        let svg = trace_to_svg(&image, &trace_settings_for_preset(QualityPreset::Figma)).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("fill=\""));
    }
}
