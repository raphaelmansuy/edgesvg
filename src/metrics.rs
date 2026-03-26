use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, Pixel, Rgba, RgbaImage};
use resvg::{tiny_skia, usvg};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct QualityMetrics {
    pub ssim: f64,
    pub psnr: f64,
    pub mae: f64,
    pub file_size: usize,
    pub path_count: usize,
}

pub fn compute_metrics(original: &DynamicImage, svg: &str) -> Result<QualityMetrics> {
    let original = flatten_on_white(&original.to_rgba8());
    let rendered = render_svg_to_image(svg, original.width(), original.height())?;
    let rendered = flatten_on_white(&rendered);

    let mut mse = 0.0;
    let mut mae = 0.0;
    let mut original_gray = Vec::with_capacity((original.width() * original.height()) as usize);
    let mut rendered_gray = Vec::with_capacity((rendered.width() * rendered.height()) as usize);

    for (left, right) in original.pixels().zip(rendered.pixels()) {
        let [lr, lg, lb, _] = left.0;
        let [rr, rg, rb, _] = right.0;
        for (a, b) in [(lr, rr), (lg, rg), (lb, rb)] {
            let delta = f64::from(a) - f64::from(b);
            mse += delta * delta;
            mae += delta.abs();
        }

        original_gray.push(luma(left));
        rendered_gray.push(luma(right));
    }

    let denom = (original.width() as f64 * original.height() as f64 * 3.0).max(1.0);
    mse /= denom;
    mae /= denom;
    let psnr = if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * ((255.0 * 255.0) / mse).log10()
    };

    Ok(QualityMetrics {
        ssim: ssim(&original_gray, &rendered_gray),
        psnr,
        mae,
        file_size: svg.as_bytes().len(),
        path_count: svg.matches("<path").count(),
    })
}

pub fn render_svg_to_image(svg: &str, width: u32, height: u32) -> Result<RgbaImage> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| anyhow!("invalid svg: {e}"))?;

    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or_else(|| anyhow!("cannot allocate pixmap"))?;
    let base_width = tree.size().width().max(1.0);
    let base_height = tree.size().height().max(1.0);
    let transform =
        tiny_skia::Transform::from_scale(width as f32 / base_width, height as f32 / base_height);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    RgbaImage::from_raw(width, height, pixmap.data().to_vec())
        .context("unable to build rendered image")
}

pub fn render_svg_file_to_png(input: &std::path::Path, output: &std::path::Path) -> Result<()> {
    let mut options = usvg::Options {
        resources_dir: input
            .canonicalize()
            .ok()
            .and_then(|path| path.parent().map(|p| p.to_path_buf())),
        ..usvg::Options::default()
    };
    options.fontdb_mut().load_system_fonts();
    let data = std::fs::read(input)?;
    let tree = usvg::Tree::from_data(&data, &options).map_err(|e| anyhow!("invalid svg: {e}"))?;
    let size = tree.size().to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow!("cannot allocate pixmap"))?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.save_png(output)?;
    Ok(())
}

fn flatten_on_white(image: &RgbaImage) -> RgbaImage {
    let mut out = RgbaImage::new(image.width(), image.height());
    for (target, source) in out.pixels_mut().zip(image.pixels()) {
        let alpha = f64::from(source[3]) / 255.0;
        let blend = |channel: u8| -> u8 {
            let channel = f64::from(channel);
            ((channel * alpha) + 255.0 * (1.0 - alpha))
                .round()
                .clamp(0.0, 255.0) as u8
        };
        *target = Rgba([blend(source[0]), blend(source[1]), blend(source[2]), 255]);
    }
    out
}

fn luma(pixel: &Rgba<u8>) -> f64 {
    let channels = pixel.channels();
    0.299 * f64::from(channels[0]) + 0.587 * f64::from(channels[1]) + 0.114 * f64::from(channels[2])
}

fn ssim(left: &[f64], right: &[f64]) -> f64 {
    let n = left.len().min(right.len());
    if n == 0 {
        return 0.0;
    }

    let mean_left = left.iter().take(n).sum::<f64>() / n as f64;
    let mean_right = right.iter().take(n).sum::<f64>() / n as f64;

    let mut var_left = 0.0;
    let mut var_right = 0.0;
    let mut covariance = 0.0;

    for (&a, &b) in left.iter().zip(right.iter()).take(n) {
        var_left += (a - mean_left).powi(2);
        var_right += (b - mean_right).powi(2);
        covariance += (a - mean_left) * (b - mean_right);
    }

    let denom = (n as f64 - 1.0).max(1.0);
    var_left /= denom;
    var_right /= denom;
    covariance /= denom;

    let c1 = (0.01_f64 * 255.0).powi(2);
    let c2 = (0.03_f64 * 255.0).powi(2);
    let value = ((2.0 * mean_left * mean_right + c1) * (2.0 * covariance + c2))
        / ((mean_left.powi(2) + mean_right.powi(2) + c1) * (var_left + var_right + c2));
    value.clamp(0.0, 1.0)
}
