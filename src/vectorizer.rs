use std::fmt;

use anyhow::{anyhow, Result};
use fastrand::Rng;
use image::RgbaImage;
use visioncortex::color_clusters::{KeyingAction, Runner, RunnerConfig, HIERARCHICAL_MAX};
use visioncortex::{
    BinaryImage, BoundingRect, Color, ColorImage, ColorName, CompoundPath, PointF64,
};

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
    elements: Vec<SvgElement>,
    width: usize,
    height: usize,
    path_precision: Option<u32>,
}

#[derive(Debug, Clone)]
enum SvgElement {
    Path(SvgPath),
    Primitive(SvgPrimitive),
}

#[derive(Debug, Clone)]
struct SvgPath {
    path: CompoundPath,
    color: Color,
}

#[derive(Debug, Clone)]
enum SvgPrimitive {
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: Color,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
        color: Color,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
        color: Color,
    },
}

impl SvgFile {
    fn new(width: usize, height: usize, path_precision: Option<u32>) -> Self {
        Self {
            elements: Vec::new(),
            width,
            height,
            path_precision,
        }
    }

    fn add_path(&mut self, path: CompoundPath, color: Color) {
        self.elements
            .push(SvgElement::Path(SvgPath { path, color }));
    }

    fn add_primitive(&mut self, primitive: SvgPrimitive) {
        self.elements.push(SvgElement::Primitive(primitive));
    }
}

impl fmt::Display for SvgFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
        writeln!(f, r#"<!-- Generator: edgesvg internal vectorizer -->"#)?;
        writeln!(
            f,
            r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="{}" height="{}">"#,
            self.width, self.height
        )?;

        for element in &self.elements {
            match element {
                SvgElement::Path(path) => path.fmt_with_precision(f, self.path_precision)?,
                SvgElement::Primitive(primitive) => {
                    primitive.fmt_with_precision(f, self.path_precision)?
                }
            }
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

impl SvgPrimitive {
    fn fmt_with_precision(
        &self,
        f: &mut fmt::Formatter<'_>,
        precision: Option<u32>,
    ) -> fmt::Result {
        match self {
            Self::Rect {
                x,
                y,
                width,
                height,
                color,
            } => writeln!(
                f,
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
                format_number(*x, precision),
                format_number(*y, precision),
                format_number(*width, precision),
                format_number(*height, precision),
                color.to_hex_string(),
            ),
            Self::Circle { cx, cy, r, color } => writeln!(
                f,
                r#"<circle cx="{}" cy="{}" r="{}" fill="{}"/>"#,
                format_number(*cx, precision),
                format_number(*cy, precision),
                format_number(*r, precision),
                color.to_hex_string(),
            ),
            Self::Ellipse {
                cx,
                cy,
                rx,
                ry,
                color,
            } => writeln!(
                f,
                r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}"/>"#,
                format_number(*cx, precision),
                format_number(*cy, precision),
                format_number(*rx, precision),
                format_number(*ry, precision),
                color.to_hex_string(),
            ),
        }
    }
}

fn format_number(value: f64, precision: Option<u32>) -> String {
    let precision = precision.unwrap_or(4) as usize;
    let mut formatted = format!("{value:.precision$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }
    if formatted == "-0" {
        "0".to_string()
    } else {
        formatted
    }
}

pub(crate) fn trace_to_svg(image: &RgbaImage, settings: &TraceSettings) -> Result<String> {
    let config = ConversionConfig::from_trace_settings(settings);
    let binary_fill = if matches!(config.color_mode, ColorMode::Binary) {
        detect_binary_fill_color(image)
    } else {
        None
    };
    let color_image = ColorImage {
        pixels: image.clone().into_raw(),
        width: image.width() as usize,
        height: image.height() as usize,
    };
    let svg = convert(color_image, config).map_err(|error| anyhow!(error))?;
    let mut svg = svg.to_string();
    if let Some((r, g, b)) = binary_fill {
        let replacement = format!("fill=\"#{r:02x}{g:02x}{b:02x}\"");
        svg = svg.replace(r#"fill="black""#, &replacement);
        svg = svg.replace("fill=\"#000000\"", &replacement);
    }
    Ok(svg)
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
        let cluster_rect = cluster.rect;
        let cluster_width = cluster_rect.width() as usize;
        let cluster_height = cluster_rect.height() as usize;
        let cluster_area = cluster.area();
        if should_attempt_primitive_detection(
            config.mode,
            cluster_width,
            cluster_height,
            cluster_area,
        ) {
            let cluster_image = cluster.to_image(&view);
            if let Some(primitive) =
                detect_primitive(&cluster_image, cluster_rect, cluster.residue_color())
            {
                svg.add_primitive(primitive);
                continue;
            }
        }
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
    let img = img.to_binary_image(|pixel| {
        let luma =
            (u16::from(pixel.r) * 299 + u16::from(pixel.g) * 587 + u16::from(pixel.b) * 114) / 1000;
        pixel.a > 16 && luma < 250
    });
    let width = img.width;
    let height = img.height;
    let clusters = img.to_clusters(false);

    let mut svg = SvgFile::new(width, height, config.path_precision);
    for index in 0..clusters.len() {
        let cluster = clusters.get_cluster(index);
        let cluster_size = cluster.size();
        if cluster_size >= config.filter_speckle_area() {
            let cluster_rect = cluster.rect;
            let cluster_width = cluster_rect.width() as usize;
            let cluster_height = cluster_rect.height() as usize;
            if should_attempt_primitive_detection(
                config.mode,
                cluster_width,
                cluster_height,
                cluster_size,
            ) {
                let cluster_image = cluster.to_binary_image();
                if let Some(primitive) = detect_primitive(
                    &cluster_image,
                    cluster_rect,
                    Color::color(&ColorName::Black),
                ) {
                    svg.add_primitive(primitive);
                    continue;
                }
            }
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

fn detect_binary_fill_color(image: &RgbaImage) -> Option<(u8, u8, u8)> {
    let mut counts = std::collections::HashMap::<[u8; 3], usize>::new();
    for pixel in image.pixels() {
        if pixel[3] <= 16 {
            continue;
        }
        *counts.entry([pixel[0], pixel[1], pixel[2]]).or_default() += 1;
    }

    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| (color[0], color[1], color[2]))
}

fn should_attempt_primitive_detection(
    mode: TraceMode,
    width: usize,
    height: usize,
    area: usize,
) -> bool {
    if !matches!(mode, TraceMode::Polygon) {
        return false;
    }

    let bbox_area = width.saturating_mul(height);
    if bbox_area < 16 || area < 8 {
        return false;
    }

    if area == bbox_area {
        return true;
    }

    width >= 4
        && height >= 4
        && (area * 100 >= bbox_area * 94
            || (area * 100 >= bbox_area * 55 && area * 100 <= bbox_area * 90))
}

fn detect_primitive(mask: &BinaryImage, rect: BoundingRect, color: Color) -> Option<SvgPrimitive> {
    if mask.width == 0 || mask.height == 0 {
        return None;
    }

    let bbox_area = mask.width * mask.height;
    let area = mask.area() as usize;
    let fill_ratio = area as f64 / bbox_area as f64;

    if area == bbox_area || (fill_ratio >= 0.94 && !has_internal_holes(mask)) {
        return Some(SvgPrimitive::Rect {
            x: rect.left as f64,
            y: rect.top as f64,
            width: mask.width as f64,
            height: mask.height as f64,
            color,
        });
    }

    if mask.width < 4 || mask.height < 4 || bbox_area < 16 || area < 8 {
        return None;
    }

    if !(0.55..=0.90).contains(&fill_ratio) || has_internal_holes(mask) {
        return None;
    }

    let allowed_mismatch = (bbox_area / 40).max(2);
    let mismatch = ellipse_mask_difference(mask, allowed_mismatch);
    if mismatch > allowed_mismatch {
        return None;
    }

    let cx = rect.left as f64 + mask.width as f64 / 2.0;
    let cy = rect.top as f64 + mask.height as f64 / 2.0;
    let rx = mask.width as f64 / 2.0;
    let ry = mask.height as f64 / 2.0;
    let aspect_delta =
        (mask.width as f64 - mask.height as f64).abs() / mask.width.max(mask.height) as f64;

    if aspect_delta <= 0.08 {
        Some(SvgPrimitive::Circle {
            cx,
            cy,
            r: rx.min(ry),
            color,
        })
    } else {
        Some(SvgPrimitive::Ellipse {
            cx,
            cy,
            rx,
            ry,
            color,
        })
    }
}

fn has_internal_holes(mask: &BinaryImage) -> bool {
    mask.negative().to_clusters(false).iter().any(|cluster| {
        cluster.rect.left > 0
            && cluster.rect.top > 0
            && cluster.rect.right < mask.width as i32
            && cluster.rect.bottom < mask.height as i32
    })
}

fn ellipse_mask_difference(mask: &BinaryImage, allowed_mismatch: usize) -> usize {
    let cx = mask.width as f64 / 2.0;
    let cy = mask.height as f64 / 2.0;
    let rx = (mask.width as f64 / 2.0).max(1.0);
    let ry = (mask.height as f64 / 2.0).max(1.0);
    let mut diff = 0usize;

    for y in 0..mask.height {
        for x in 0..mask.width {
            let dx = (x as f64 + 0.5 - cx) / rx;
            let dy = (y as f64 + 0.5 - cy) / ry;
            let expected = dx * dx + dy * dy <= 1.0;
            if mask.get_pixel(x, y) != expected {
                diff += 1;
                if diff > allowed_mismatch {
                    return diff;
                }
            }
        }
    }

    diff
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{detect_primitive, trace_to_svg};
    use crate::preprocess::trace_settings_for_preset;
    use crate::types::{QualityPreset, TraceMode, TraceSettings};
    use visioncortex::{BinaryImage, BoundingRect};

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
        assert!(svg.contains("<path") || svg.contains("<rect"));
        assert!(svg.contains("fill=\""));
    }

    #[test]
    fn binary_trace_preserves_monochrome_fill_color() {
        let mut image = RgbaImage::new(16, 16);
        for y in 4..12 {
            for x in 4..12 {
                image.put_pixel(x, y, Rgba([103, 0, 235, 255]));
            }
        }

        let settings = TraceSettings {
            color_mode: "binary",
            hierarchical: "stacked",
            mode: TraceMode::Spline,
            filter_speckle: 0,
            color_precision: 6,
            layer_difference: 0,
            length_threshold: 3.5,
            corner_threshold: 45,
            max_iterations: 10,
            splice_threshold: 45,
            path_precision: 5,
            optimizer_precision: 1,
        };
        let svg = trace_to_svg(&image, &settings).unwrap();
        let svg = svg.to_ascii_lowercase();
        assert!(svg.contains("fill=\"#6700eb\""));
        assert!(!svg.contains(r#"fill="black""#));
    }

    #[test]
    fn primitive_detection_emits_rect_for_filled_box() {
        let mut mask = BinaryImage::new_w_h(8, 6);
        for y in 0..6 {
            for x in 0..8 {
                mask.set_pixel(x, y, true);
            }
        }

        let primitive = detect_primitive(
            &mask,
            BoundingRect::new_x_y_w_h(4, 3, 8, 6),
            visioncortex::Color::color(&visioncortex::ColorName::Black),
        )
        .expect("expected rect primitive");

        assert!(matches!(primitive, super::SvgPrimitive::Rect { .. }));
    }

    #[test]
    fn primitive_detection_emits_rect_for_near_filled_box() {
        let mut mask = BinaryImage::new_w_h(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                if (x, y) != (0, 0) && (x, y) != (9, 9) {
                    mask.set_pixel(x, y, true);
                }
            }
        }

        let primitive = detect_primitive(
            &mask,
            BoundingRect::new_x_y_w_h(4, 3, 10, 10),
            visioncortex::Color::color(&visioncortex::ColorName::Black),
        )
        .expect("expected rect primitive");

        assert!(matches!(primitive, super::SvgPrimitive::Rect { .. }));
    }

    #[test]
    fn primitive_gate_skips_non_polygon_mode() {
        assert!(!super::should_attempt_primitive_detection(
            TraceMode::Spline,
            16,
            16,
            200
        ));
    }

    #[test]
    fn trace_to_svg_emits_circle_for_simple_disc() {
        let mut image = RgbaImage::new(12, 12);
        let cx = 6.0;
        let cy = 6.0;
        let r = 4.0;
        for y in 0..12 {
            for x in 0..12 {
                let dx = x as f64 + 0.5 - cx;
                let dy = y as f64 + 0.5 - cy;
                if dx * dx + dy * dy <= r * r {
                    image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            }
        }

        let settings = TraceSettings {
            color_mode: "binary",
            hierarchical: "stacked",
            mode: TraceMode::Polygon,
            filter_speckle: 0,
            color_precision: 8,
            layer_difference: 0,
            length_threshold: 2.0,
            corner_threshold: 15,
            max_iterations: 20,
            splice_threshold: 30,
            path_precision: 4,
            optimizer_precision: 1,
        };
        let svg = trace_to_svg(&image, &settings).unwrap();
        assert!(svg.contains("<circle"));
        assert!(!svg.contains("<path"));
    }
}
