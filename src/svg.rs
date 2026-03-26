use regex::Regex;
use roxmltree::Document;

pub fn optimize_svg(svg: &str, precision: u32) -> String {
    let parsed = match Document::parse(svg) {
        Ok(parsed) if parsed.root_element().has_tag_name("svg") => parsed,
        _ => return svg.to_owned(),
    };

    let path_attr_re = Regex::new(r#"d="([^"]+)""#).expect("valid path attribute regex");
    let optimized = path_attr_re.replace_all(svg, |caps: &regex::Captures<'_>| {
        let d = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        format!("d=\"{}\"", minify_path_data(d, precision))
    });

    merge_same_fill_direct_paths(&minify_svg_fallback(&optimized), &parsed)
}

fn merge_same_fill_direct_paths(svg: &str, parsed: &Document<'_>) -> String {
    let root = parsed.root_element();
    let direct_path_children = root
        .children()
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();

    if direct_path_children.is_empty()
        || direct_path_children
            .iter()
            .any(|node| !node.has_tag_name("path"))
    {
        return svg.to_owned();
    }

    let mut root_open = String::from("<svg");
    for attr in root.attributes() {
        root_open.push(' ');
        root_open.push_str(attr.name());
        root_open.push_str("=\"");
        root_open.push_str(attr.value());
        root_open.push('"');
    }
    root_open.push('>');

    let mut merged = Vec::<(String, String)>::new();
    for path in direct_path_children {
        let attrs = path
            .attributes()
            .filter(|attr| attr.name() != "d")
            .map(|attr| format!(r#"{}="{}""#, attr.name(), attr.value()))
            .collect::<Vec<_>>()
            .join(" ");
        let d = path.attribute("d").unwrap_or_default();

        if let Some((_, existing_d)) = merged
            .iter_mut()
            .find(|(existing_attrs, _)| existing_attrs == &attrs)
        {
            if !existing_d.is_empty() && !d.is_empty() {
                existing_d.push(' ');
            }
            existing_d.push_str(d);
        } else {
            merged.push((attrs, d.to_string()));
        }
    }

    let mut out = root_open;
    for (attrs, d) in merged {
        out.push_str("<path");
        if !attrs.is_empty() {
            out.push(' ');
            out.push_str(&attrs);
        }
        out.push_str(r#" d=""#);
        out.push_str(&d);
        out.push_str(r#""/>"#);
    }
    out.push_str("</svg>");
    out
}

pub fn minify_path_data(d: &str, precision: u32) -> String {
    let num_re = Regex::new(r"-?\d+(?:\.\d+)?").expect("valid numeric regex");
    let mut rounded = num_re
        .replace_all(d, |caps: &regex::Captures<'_>| {
            let value = caps
                .get(0)
                .and_then(|m| m.as_str().parse::<f64>().ok())
                .unwrap_or(0.0);
            if (value - value.round()).abs() < 0.01 {
                format!("{}", value.round() as i64)
            } else {
                format!("{:.*}", precision as usize, value)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            }
        })
        .to_string();

    let ws_re = Regex::new(r"\s+").expect("valid whitespace regex");
    rounded = ws_re.replace_all(rounded.trim(), " ").to_string();

    let cmd_re = Regex::new(r"([MmLlHhVvCcSsQqTtAaZz])\s+").expect("valid path command regex");
    rounded = cmd_re.replace_all(&rounded, "$1").to_string();
    rounded
        .replace(", ", ",")
        .replace(" ,", ",")
        .replace(' ', ",")
}

fn minify_svg_fallback(svg: &str) -> String {
    let comment_re = Regex::new(r"<!--.*?-->").expect("valid comment regex");
    let ws_re = Regex::new(r">\s+<").expect("valid tag whitespace regex");
    let repeated_ws = Regex::new(r"\s{2,}").expect("valid repeated whitespace regex");
    let trimmed = comment_re.replace_all(svg, "");
    let collapsed = ws_re.replace_all(&trimmed, "><");
    repeated_ws.replace_all(collapsed.trim(), " ").to_string()
}

#[cfg(test)]
mod tests {
    use super::{minify_path_data, optimize_svg};

    #[test]
    fn minifies_numeric_precision_without_dropping_commands() {
        let d = "M 0.12345 0.98765 L 10.111 20.222 Z";
        let minified = minify_path_data(d, 1);
        assert!(minified.starts_with("M"));
        assert!(minified.contains("10.1"));
        assert!(minified.ends_with("Z"));
    }

    #[test]
    fn optimizer_preserves_svg_structure() {
        let svg = r##"<svg width="10" height="10" xmlns="http://www.w3.org/2000/svg"><g><path fill="#000" d="M 0.12345 0.98765 L 10.111 20.222"/></g></svg>"##;
        let optimized = optimize_svg(svg, 1);
        assert!(optimized.contains("<svg"));
        assert!(optimized.contains("<g>"));
        assert!(optimized.contains("<path"));
    }

    #[test]
    fn optimizer_merges_direct_paths_with_same_attributes() {
        let svg = r##"<svg width="10" height="10" xmlns="http://www.w3.org/2000/svg"><path fill="#000" d="M 0 0 L 1 1"/><path fill="#000" d="M 2 2 L 3 3"/></svg>"##;
        let optimized = optimize_svg(svg, 1);
        assert_eq!(optimized.matches("<path").count(), 1);
    }
}
