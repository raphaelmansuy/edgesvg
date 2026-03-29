use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct LinearGradientStop {
    pub offset: f64,
    pub color: [u8; 3],
}

#[derive(Debug, Clone)]
pub struct LinearGradientSpec {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub stops: Vec<LinearGradientStop>,
}

fn path_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"d="([^"]+)""#).expect("valid path attribute regex"))
}

fn num_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"-?\d*\.?\d+").expect("valid numeric regex"))
}

fn repeated_ws_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+").expect("valid repeated whitespace regex"))
}

fn cmd_space_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"([MmLlHhVvCcSsQqTtAaZz])\s+").expect("valid command spacing regex")
    })
}

fn comma_space_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s*,\s*").expect("valid comma spacing regex"))
}

fn paint_attr_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(fill|stroke)="([^"]+)""#).expect("valid paint attribute regex")
    })
}

fn rgb_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"rgb\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)").expect("valid rgb regex")
    })
}

fn hex_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$").expect("valid hex regex")
    })
}

fn removable_attrs_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\s(?:id|class|style)="[^"]*""#).expect("valid removable attributes regex")
    })
}

fn fill_opacity_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\sfill-opacity="(?:1|1\.0)""#).expect("valid opacity regex"))
}

fn stroke_opacity_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\sstroke-opacity="(?:1|1\.0)""#).expect("valid opacity regex"))
}

fn opacity_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\sopacity="(?:1|1\.0)""#).expect("valid opacity regex"))
}

fn zero_translate_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\stransform="translate\(0(?:\.0+)?,0(?:\.0+)?\)""#)
            .expect("valid zero translate regex")
    })
}

fn stroke_none_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\sstroke="none""#).expect("valid stroke none attribute regex"))
}

fn path_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<path\b([^>]*)/>"#).expect("valid self-closing path regex"))
}

fn d_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\sd="([^"]*)""#).expect("valid d attribute regex"))
}

fn ns_prefix_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"ns\d+:").expect("valid namespace prefix regex"))
}

fn xmlns_ns_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"\sxmlns:ns\d+="[^"]*""#).expect("valid namespace declaration regex")
    })
}

fn xml_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<\?xml[^?]*\?>\s*"#).expect("valid xml declaration regex"))
}

fn comment_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"<!--.*?-->"#).expect("valid comment regex"))
}

fn between_tags_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#">\s+<"#).expect("valid between tags regex"))
}

fn space_self_close_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\s+/>"#).expect("valid self close regex"))
}

fn space_close_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"\s+>"#).expect("valid closing tag spacing regex"))
}

fn svg_open_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?s)^.*?<svg\b[^>]*>"#).expect("valid svg open regex"))
}

fn svg_close_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"(?s)</svg>\s*$"#).expect("valid svg close regex"))
}

fn fill_attr_only_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"fill="[^"]+""#).expect("valid fill attribute regex"))
}

pub fn optimize_svg(svg: &str, precision: u32) -> String {
    if !svg.contains("<svg") || !svg.contains("</svg>") {
        return svg.to_owned();
    }
    let has_namespace_artifacts = svg.contains("xmlns:ns") || ns_prefix_re().is_match(svg);

    let optimized = round_path_coordinates(svg, precision);
    let optimized = optimize_paint_attributes(&optimized);
    let optimized = merge_same_style_paths(&optimized);
    let optimized = remove_redundant_attributes(&optimized);
    let optimized = clean_namespaces(&optimized, has_namespace_artifacts);
    final_cleanup(&optimized)
}

pub fn wrap_svg_with_gradient_background(
    width: u32,
    height: u32,
    gradient: &LinearGradientSpec,
    overlay_svg: &str,
) -> String {
    let content = extract_svg_content(overlay_svg);
    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">"#
    ));
    out.push_str("<defs>");
    append_linear_gradient_def(&mut out, "bgGradient", gradient);
    out.push_str("</defs>");
    out.push_str(&format!(
        r#"<rect width="{width}" height="{height}" fill="url(#bgGradient)"/>"#
    ));
    out.push_str(&content);
    out.push_str("</svg>");
    out
}

pub fn wrap_svg_with_gradient_surface(
    width: u32,
    height: u32,
    gradient: &LinearGradientSpec,
    surface_svg: &str,
    overlay_svg: &str,
) -> String {
    let surface = apply_gradient_fill(&extract_svg_content(surface_svg), "surfaceGradient");
    let overlay = extract_svg_content(overlay_svg);
    let mut out = String::new();
    out.push_str(&format!(
        r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">"#
    ));
    out.push_str("<defs>");
    append_linear_gradient_def(&mut out, "surfaceGradient", gradient);
    out.push_str("</defs>");
    out.push_str(&surface);
    out.push_str(&overlay);
    out.push_str("</svg>");
    out
}

fn extract_svg_content(svg: &str) -> String {
    let stripped = xml_re().replace_all(svg, "");
    let stripped = comment_re().replace_all(&stripped, "");
    let stripped = svg_open_re().replace(&stripped, "");
    let stripped = svg_close_re().replace(&stripped, "");
    stripped.trim().to_string()
}

fn append_linear_gradient_def(out: &mut String, id: &str, gradient: &LinearGradientSpec) {
    out.push_str(&format!(
        r#"<linearGradient id="{id}" x1="{}" y1="{}" x2="{}" y2="{}" gradientUnits="userSpaceOnUse">"#,
        format_number(gradient.x1),
        format_number(gradient.y1),
        format_number(gradient.x2),
        format_number(gradient.y2),
    ));
    for stop in &gradient.stops {
        out.push_str(&format!(
            "<stop offset=\"{}%\" stop-color=\"#{:02x}{:02x}{:02x}\"/>",
            format_number((stop.offset * 100.0).clamp(0.0, 100.0)),
            stop.color[0],
            stop.color[1],
            stop.color[2],
        ));
    }
    out.push_str("</linearGradient>");
}

fn apply_gradient_fill(svg_content: &str, gradient_id: &str) -> String {
    fill_attr_only_re()
        .replace_all(svg_content, format!(r#"fill="url(#{gradient_id})""#))
        .to_string()
}

fn format_number(value: f64) -> String {
    let mut formatted = format!("{value:.4}");
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

fn round_path_coordinates(svg: &str, precision: u32) -> String {
    path_attr_re()
        .replace_all(svg, |caps: &regex::Captures<'_>| {
            let d = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            format!(r#"d="{}""#, round_path_data(d, precision))
        })
        .into_owned()
}

fn round_path_data(d: &str, precision: u32) -> String {
    let rounded = num_re()
        .replace_all(d, |caps: &regex::Captures<'_>| {
            let value = caps
                .get(0)
                .and_then(|m| m.as_str().parse::<f64>().ok())
                .unwrap_or(0.0);
            if value.fract().abs() < f64::EPSILON {
                format!("{}", value as i64)
            } else {
                format!("{:.*}", precision as usize, value)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            }
        })
        .into_owned();
    simplify_path_data(&rounded)
}

fn simplify_path_data(d: &str) -> String {
    let simplified = repeated_ws_re().replace_all(d.trim(), " ");
    let simplified = cmd_space_re().replace_all(&simplified, "$1");
    comma_space_re().replace_all(&simplified, ",").into_owned()
}

fn optimize_paint_attributes(svg: &str) -> String {
    paint_attr_re()
        .replace_all(svg, |caps: &regex::Captures<'_>| {
            let attr = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let value = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
            format!(r#"{attr}="{}""#, optimize_color(value))
        })
        .into_owned()
}

fn optimize_color(color: &str) -> String {
    let mut color = color.trim().to_ascii_lowercase();
    if let Some(caps) = rgb_re().captures(&color) {
        let parse = |index| {
            caps.get(index)
                .and_then(|m| m.as_str().parse::<u8>().ok())
                .unwrap_or(0)
        };
        color = format!("#{:02x}{:02x}{:02x}", parse(1), parse(2), parse(3));
    }

    if let Some(caps) = hex_re().captures(&color) {
        let r = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let g = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let b = caps.get(3).map(|m| m.as_str()).unwrap_or_default();
        if r.as_bytes()[0] == r.as_bytes()[1]
            && g.as_bytes()[0] == g.as_bytes()[1]
            && b.as_bytes()[0] == b.as_bytes()[1]
        {
            color = format!("#{}{}{}", &r[0..1], &g[0..1], &b[0..1],);
        }
    }

    match color.as_str() {
        "#000" => "black".to_string(),
        "#fff" => "white".to_string(),
        "#f00" => "red".to_string(),
        "#0f0" => "lime".to_string(),
        "#00f" => "blue".to_string(),
        "#ff0" => "yellow".to_string(),
        "#0ff" => "cyan".to_string(),
        "#f0f" => "magenta".to_string(),
        _ => color,
    }
}

fn remove_redundant_attributes(svg: &str) -> String {
    let mut optimized = svg.to_owned();
    optimized = removable_attrs_re()
        .replace_all(&optimized, "")
        .into_owned();
    optimized = fill_opacity_re().replace_all(&optimized, "").into_owned();
    optimized = stroke_opacity_re().replace_all(&optimized, "").into_owned();
    optimized = opacity_re().replace_all(&optimized, "").into_owned();
    optimized = zero_translate_re().replace_all(&optimized, "").into_owned();
    stroke_none_re().replace_all(&optimized, "").into_owned()
}

fn merge_same_style_paths(svg: &str) -> String {
    let mut segments = Vec::new();
    let mut last_end = 0usize;

    for caps in path_re().captures_iter(svg) {
        let full = match caps.get(0) {
            Some(m) => m,
            None => continue,
        };
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        if full.start() > last_end {
            segments.push(Segment::Text(svg[last_end..full.start()].to_string()));
        }
        let style = d_re().replace(attrs, "").to_string();
        let style = normalize_attr_whitespace(&style);
        let d = d_re()
            .captures(attrs)
            .and_then(|d_caps| d_caps.get(1).map(|m| m.as_str().to_string()))
            .unwrap_or_default();
        segments.push(Segment::Path {
            raw: full.as_str().to_string(),
            d,
            style,
        });
        last_end = full.end();
    }

    if segments.is_empty() {
        return svg.to_owned();
    }

    if last_end < svg.len() {
        segments.push(Segment::Text(svg[last_end..].to_string()));
    }

    let mut first_by_style: HashMap<String, usize> = HashMap::new();
    let mut merged_paths: HashMap<usize, Vec<String>> = HashMap::new();
    let mut remove = vec![false; segments.len()];

    for (index, segment) in segments.iter().enumerate() {
        let Segment::Path { d, style, .. } = segment else {
            continue;
        };
        if let Some(first_index) = first_by_style.get(style) {
            merged_paths
                .entry(*first_index)
                .or_default()
                .push(d.clone());
            remove[index] = true;
        } else {
            first_by_style.insert(style.clone(), index);
        }
    }

    let mut output = String::new();
    for (index, segment) in segments.into_iter().enumerate() {
        if remove[index] {
            continue;
        }
        match segment {
            Segment::Text(text) => output.push_str(&text),
            Segment::Path { raw, d, .. } => {
                let merged = merged_paths.get(&index);
                let merged_d = merged
                    .map(|paths| {
                        let mut combined = Vec::with_capacity(paths.len() + 1);
                        combined.push(d.clone());
                        combined.extend(paths.iter().cloned());
                        combined.join(" ")
                    })
                    .unwrap_or(d);
                output.push_str(&d_re().replace(&raw, format!(r#" d="{}""#, merged_d)));
            }
        }
    }

    output
}

fn normalize_attr_whitespace(attrs: &str) -> String {
    repeated_ws_re().replace_all(attrs.trim(), " ").into_owned()
}

enum Segment {
    Text(String),
    Path {
        raw: String,
        d: String,
        style: String,
    },
}

fn clean_namespaces(svg: &str, has_namespace_artifacts: bool) -> String {
    let mut cleaned = svg.to_owned();
    if has_namespace_artifacts {
        cleaned = ns_prefix_re().replace_all(&cleaned, "").into_owned();
        cleaned = xmlns_ns_re().replace_all(&cleaned, "").into_owned();
    }

    if !cleaned.contains("xmlns=") {
        if cleaned.contains("<svg ") {
            cleaned = cleaned.replacen("<svg ", r#"<svg xmlns="http://www.w3.org/2000/svg" "#, 1);
        } else if cleaned.contains("<svg>") {
            cleaned = cleaned.replacen("<svg>", r#"<svg xmlns="http://www.w3.org/2000/svg">"#, 1);
        }
    }

    cleaned
}

fn final_cleanup(svg: &str) -> String {
    let cleaned = xml_re().replace_all(svg, "");
    let cleaned = comment_re().replace_all(&cleaned, "");
    let cleaned = repeated_ws_re().replace_all(&cleaned, " ");
    let cleaned = between_tags_re().replace_all(&cleaned, "><");
    let cleaned = space_self_close_re().replace_all(&cleaned, "/>");
    let cleaned = space_close_re().replace_all(&cleaned, ">");
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        optimize_color, optimize_svg, round_path_data, wrap_svg_with_gradient_surface,
        LinearGradientSpec, LinearGradientStop,
    };

    #[test]
    fn rounds_numeric_precision_like_reference_optimizer() {
        let d = "M 0.12345 0.98765 L 10.111 20.222 Z";
        let rounded = round_path_data(d, 1);
        assert_eq!(rounded, "M0.1 1 L10.1 20.2 Z");
    }

    #[test]
    fn optimizer_keeps_invalid_input_unchanged() {
        assert_eq!(optimize_svg("not valid svg", 1), "not valid svg");
    }

    #[test]
    fn optimizer_cleans_colors_and_redundant_attributes() {
        let svg = r#"<svg><path id="a" class="b" style="x:y" fill="rgb(255,0,0)" stroke="none" opacity="1" d="M 0.123 1.987"/></svg>"#;
        let optimized = optimize_svg(svg, 1);
        assert!(optimized.contains(r#"fill="red""#));
        assert!(!optimized.contains("stroke=\"none\""));
        assert!(!optimized.contains("opacity=\"1\""));
        assert!(!optimized.contains("id=\"a\""));
        assert!(optimized.contains(r#"d="M0.1 2""#));
    }

    #[test]
    fn color_optimizer_matches_reference_shortening_rules() {
        assert_eq!(optimize_color("rgb(255,0,0)"), "red");
        assert_eq!(optimize_color("#ff00ff"), "magenta");
        assert_eq!(optimize_color("#112233"), "#123");
    }

    #[test]
    fn optimizer_merges_same_style_paths_and_removes_zero_translate() {
        let svg = r##"<svg><path d="M 0 0" fill="#000000" transform="translate(0,0)"/><path d="M 1 1" fill="#000000" transform="translate(0,0)"/><path d="M 2 2" fill="#ffffff"/></svg>"##;
        let optimized = optimize_svg(svg, 1);
        assert_eq!(optimized.matches("<path").count(), 2);
        assert!(optimized.contains(r#"d="M0 0 M1 1""#));
        assert!(!optimized.contains(r#"transform="translate(0,0)""#));
    }

    #[test]
    fn wrap_svg_with_gradient_surface_replaces_mask_fill_only() {
        let gradient = LinearGradientSpec {
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 0.0,
            stops: vec![
                LinearGradientStop {
                    offset: 0.0,
                    color: [0, 0, 0],
                },
                LinearGradientStop {
                    offset: 1.0,
                    color: [255, 255, 255],
                },
            ],
        };
        let surface_svg = r##"<svg><path d="M0 0H10V10H0Z" fill="#000000"/></svg>"##;
        let overlay_svg = r##"<svg><path d="M1 1H3V3H1Z" fill="#ff0000"/></svg>"##;

        let wrapped =
            wrap_svg_with_gradient_surface(10, 10, &gradient, surface_svg, overlay_svg);

        assert!(wrapped.contains(r#"id="surfaceGradient""#));
        assert!(wrapped.contains(r#"fill="url(#surfaceGradient)""#));
        assert!(wrapped.contains(r##"fill="#ff0000""##));
    }
}
