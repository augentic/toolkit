use anyhow::{Result, bail};

use crate::config::Values;
use crate::manifest::Header;

// `{{key}}` with a kebab-case key and no spaces, so a GitHub expression such as
// `${{ github.workflow }}` passes through as text.
fn placeholder_at(text: &str) -> Option<(&str, usize)> {
    let rest = text.strip_prefix("{{")?;
    let close = rest.find("}}")?;
    let key = &rest[..close];
    let kebab = key.starts_with(|c: char| c.is_ascii_lowercase())
        && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    kebab.then_some((key, close + 4))
}

// The one placeholder a text consists of, as a manifest target may.
pub fn sole_placeholder(text: &str) -> Option<&str> {
    let (key, len) = placeholder_at(text)?;
    (len == text.len()).then_some(key)
}

// Renders a template: placeholders substituted, a line whose placeholder renders
// empty dropped, and a YAML key left with no children dropped with them.
pub fn render(template: &str, values: &Values) -> Result<String> {
    // substitute, marking the lines an empty value removes
    let mut lines: Vec<Line> = Vec::new();
    for line in template.lines() {
        let indent = line.len() - line.trim_start().len();
        let mut out = String::new();
        let mut rest = line;
        let mut dropped = false;
        while let Some(at) = rest.find("{{") {
            out.push_str(&rest[..at]);
            rest = &rest[at..];
            if let Some((key, len)) = placeholder_at(rest) {
                let Some(value) = values.scalars.get(key) else {
                    bail!("unknown placeholder {{{{{key}}}}}");
                };
                if value.is_empty() {
                    dropped = true;
                }
                out.push_str(value);
                rest = &rest[len..];
            } else {
                out.push_str("{{");
                rest = &rest[2..];
            }
        }
        out.push_str(rest);
        lines.push(Line {
            indent,
            blank: line.trim().is_empty(),
            kept: (!dropped).then_some(out),
        });
    }

    // a key whose template children were all dropped goes with them
    loop {
        let mut changed = false;
        for i in 0..lines.len() {
            let Some(text) = &lines[i].kept else { continue };
            if !text.trim_end().ends_with(':') || text.trim_start().starts_with('#') {
                continue;
            }
            let indent = lines[i].indent;
            let children = lines[i + 1..]
                .iter()
                .take_while(|line| line.blank || line.indent > indent)
                .filter(|line| !line.blank);
            let mut any = false;
            let all_dropped = children.inspect(|_| any = true).all(|line| line.kept.is_none());
            if any && all_dropped {
                lines[i].kept = None;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut out = lines.into_iter().filter_map(|line| line.kept).collect::<Vec<_>>().join("\n");
    out.push('\n');
    Ok(out)
}

struct Line {
    indent: usize,
    blank: bool,
    kept: Option<String>,
}

// The one-line notice a managed whole file or stub opens with.
pub fn header(header: Header, from: &str) -> String {
    let notice = format!(
        "Managed by augentic/toolkit from {from}. Do not edit: run `make conventions-sync`."
    );
    match header {
        Header::Hash => format!("# {notice}\n\n"),
        Header::Html => format!("<!-- {notice} -->\n\n"),
        Header::None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn values(pairs: &[(&'static str, &str)]) -> Values {
        let scalars = pairs.iter().map(|(k, v)| (*k, (*v).to_owned())).collect::<BTreeMap<_, _>>();
        Values {
            scalars,
            lists: BTreeMap::new(),
        }
    }

    #[test]
    fn github_expression_passes_through() {
        let out =
            render("group: ${{ github.workflow }}-{{name}}\n", &values(&[("name", "x")])).unwrap();
        assert_eq!(out, "group: ${{ github.workflow }}-x\n");
    }

    #[test]
    fn empty_value_drops_line_and_emptied_parent() {
        let template =
            "with:\n  targets: {{targets}}\n  wasm-packages: {{wasm-packages}}\nnext: 1\n";
        let out = render(template, &values(&[("targets", ""), ("wasm-packages", "")])).unwrap();
        assert_eq!(out, "next: 1\n");
        let out = render(template, &values(&[("targets", "wasm32-wasip2"), ("wasm-packages", "")]))
            .unwrap();
        assert_eq!(out, "with:\n  targets: wasm32-wasip2\nnext: 1\n");
    }

    #[test]
    fn childless_key_stays() {
        let out =
            render("on:\n  workflow_dispatch:\n\nname: {{name}}\n", &values(&[("name", "x")]))
                .unwrap();
        assert_eq!(out, "on:\n  workflow_dispatch:\n\nname: x\n");
    }

    #[test]
    fn unknown_placeholder_is_an_error() {
        render("{{nope}}", &values(&[])).unwrap_err();
    }
}
