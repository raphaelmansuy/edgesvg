use regex::Regex;
use roxmltree::Document;

pub fn optimize_svg(svg: &str, precision: u32) -> String {
    let parsed = match Document::parse(svg) {
        Ok(parsed) if parsed.root_element().has_tag_name("svg") => parsed,
        _ => return svg.to_owned(),
    };

    let optimized = round_path_coordinates(svg, precision);
    let optimized = optimize_paint_attributes(&optimized);
    let optimized = remove_redundant_attributes(&optimized);
    let optimized = clean_namespaces(&optimized, &parsed);
    final_cleanup(&optimized)
}

fn round_path_coordinates(svg: &str, precision: u32) -> String {
    let path_attr_re = Regex::new(r#"d="([^"]+)""#).expect("valid path attribute regex");
    path_attr_re
        .replace_all(svg, |caps: &regex::Captures<'_>| {
            let d = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            format!(r#"d="{}""#, round_path_data(d, precision))
        })
        .into_owned()
}

fn round_path_data(d: &str, precision: u32) -> String {
    let num_re = Regex::new(r"-?\d*\.?\d+").expect("valid numeric regex");
    num_re
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
        .into_owned()
}

fn optimize_paint_attributes(svg: &str) -> String {
    let paint_attr_re =
        Regex::new(r#"(fill|stroke)="([^"]+)""#).expect("valid paint attribute regex");
    paint_attr_re
        .replace_all(svg, |caps: &regex::Captures<'_>| {
            let attr = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let value = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
            format!(r#"{attr}="{}""#, optimize_color(value))
        })
        .into_owned()
}

fn optimize_color(color: &str) -> String {
    let mut color = color.trim().to_ascii_lowercase();
    let rgb_re =
        Regex::new(r"rgb\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)").expect("valid rgb regex");
    if let Some(caps) = rgb_re.captures(&color) {
        let parse = |index| {
            caps.get(index)
                .and_then(|m| m.as_str().parse::<u8>().ok())
                .unwrap_or(0)
        };
        color = format!("#{:02x}{:02x}{:02x}", parse(1), parse(2), parse(3));
    }

    let hex_re =
        Regex::new(r"^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$").expect("valid hex regex");
    if let Some(caps) = hex_re.captures(&color) {
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
    let removable_attrs =
        Regex::new(r#"\s(?:id|class|style)="[^"]*""#).expect("valid removable attributes regex");
    optimized = removable_attrs.replace_all(&optimized, "").into_owned();

    for attr in ["fill-opacity", "stroke-opacity", "opacity"] {
        let pattern = format!(r#"\s{attr}="(?:1|1\.0)""#);
        let attr_re = Regex::new(&pattern).expect("valid opacity regex");
        optimized = attr_re.replace_all(&optimized, "").into_owned();
    }

    let stroke_none = Regex::new(r#"\sstroke="none""#).expect("valid stroke none attribute regex");
    stroke_none.replace_all(&optimized, "").into_owned()
}

fn clean_namespaces(svg: &str, parsed: &Document<'_>) -> String {
    let mut cleaned = svg.to_owned();
    if parsed.root_element().tag_name().namespace().is_some() {
        let ns_prefix_re = Regex::new(r"ns\d+:").expect("valid namespace prefix regex");
        cleaned = ns_prefix_re.replace_all(&cleaned, "").into_owned();

        let xmlns_ns_re =
            Regex::new(r#"\sxmlns:ns\d+="[^"]*""#).expect("valid namespace declaration regex");
        cleaned = xmlns_ns_re.replace_all(&cleaned, "").into_owned();
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
    let xml_re = Regex::new(r#"<\?xml[^?]*\?>\s*"#).expect("valid xml declaration regex");
    let comment_re = Regex::new(r#"<!--.*?-->"#).expect("valid comment regex");
    let repeated_ws = Regex::new(r#"\s+"#).expect("valid repeated whitespace regex");
    let between_tags = Regex::new(r#">\s+<"#).expect("valid between tags regex");
    let space_self_close = Regex::new(r#"\s+/>"#).expect("valid self close regex");
    let space_close = Regex::new(r#"\s+>"#).expect("valid closing tag spacing regex");

    let cleaned = xml_re.replace_all(svg, "");
    let cleaned = comment_re.replace_all(&cleaned, "");
    let cleaned = repeated_ws.replace_all(&cleaned, " ");
    let cleaned = between_tags.replace_all(&cleaned, "><");
    let cleaned = space_self_close.replace_all(&cleaned, "/>");
    let cleaned = space_close.replace_all(&cleaned, ">");
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{optimize_color, optimize_svg, round_path_data};

    #[test]
    fn rounds_numeric_precision_like_reference_optimizer() {
        let d = "M 0.12345 0.98765 L 10.111 20.222 Z";
        let rounded = round_path_data(d, 1);
        assert_eq!(rounded, "M 0.1 1 L 10.1 20.2 Z");
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
        assert!(optimized.contains(r#"d="M 0.1 2""#));
    }

    #[test]
    fn color_optimizer_matches_reference_shortening_rules() {
        assert_eq!(optimize_color("rgb(255,0,0)"), "red");
        assert_eq!(optimize_color("#ff00ff"), "magenta");
        assert_eq!(optimize_color("#112233"), "#123");
    }
}
