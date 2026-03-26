use regex::Regex;
use roxmltree::Document;
use std::collections::HashMap;

pub fn optimize_svg(svg: &str, precision: u32) -> String {
    let parsed = match Document::parse(svg) {
        Ok(parsed) if parsed.root_element().has_tag_name("svg") => parsed,
        _ => return svg.to_owned(),
    };

    let optimized = round_path_coordinates(svg, precision);
    let optimized = optimize_paint_attributes(&optimized);
    let optimized = merge_same_style_paths(&optimized);
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
    let rounded = num_re
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
    let repeated_ws = Regex::new(r"\s+").expect("valid repeated whitespace regex");
    let cmd_space =
        Regex::new(r"([MmLlHhVvCcSsQqTtAaZz])\s+").expect("valid command spacing regex");
    let comma_space = Regex::new(r"\s*,\s*").expect("valid comma spacing regex");

    let simplified = repeated_ws.replace_all(d.trim(), " ");
    let simplified = cmd_space.replace_all(&simplified, "$1");
    comma_space.replace_all(&simplified, ",").into_owned()
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

    let zero_translate = Regex::new(r#"\stransform="translate\(0(?:\.0+)?,0(?:\.0+)?\)""#)
        .expect("valid zero translate regex");
    optimized = zero_translate.replace_all(&optimized, "").into_owned();

    let stroke_none = Regex::new(r#"\sstroke="none""#).expect("valid stroke none attribute regex");
    stroke_none.replace_all(&optimized, "").into_owned()
}

fn merge_same_style_paths(svg: &str) -> String {
    let path_re = Regex::new(r#"<path\b([^>]*)/>"#).expect("valid self-closing path regex");
    let d_re = Regex::new(r#"\sd="([^"]*)""#).expect("valid d attribute regex");
    let mut segments = Vec::new();
    let mut last_end = 0usize;

    for caps in path_re.captures_iter(svg) {
        let full = match caps.get(0) {
            Some(m) => m,
            None => continue,
        };
        let attrs = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        if full.start() > last_end {
            segments.push(Segment::Text(svg[last_end..full.start()].to_string()));
        }
        let style = d_re.replace(attrs, "").to_string();
        let style = normalize_attr_whitespace(&style);
        let d = d_re
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
                output.push_str(
                    &d_re
                        .replace(&raw, format!(r#" d="{}""#, merged_d))
                        .into_owned(),
                );
            }
        }
    }

    output
}

fn normalize_attr_whitespace(attrs: &str) -> String {
    let repeated_ws = Regex::new(r"\s+").expect("valid repeated whitespace regex");
    repeated_ws.replace_all(attrs.trim(), " ").into_owned()
}

enum Segment {
    Text(String),
    Path {
        raw: String,
        d: String,
        style: String,
    },
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
}
