use std::path::Path;

use anyhow::{Result, bail};

use crate::template;

// The comment syntax a marker pair is written in: HTML comments in Markdown,
// `#` comments everywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Html,
    Hash,
}

const BEGIN: &str = "BEGIN ";
const END: &str = "END ";
// The spellings before 0.4.0, read for this release so a consumer's first
// sync respells its pairs in place instead of appending a second copy.
const LEGACY_BEGIN: &str = "conventions:begin ";
const LEGACY_END: &str = "conventions:end ";

impl Style {
    pub fn of(target: &Path) -> Self {
        match target.extension().and_then(|e| e.to_str()) {
            Some("md") => Self::Html,
            _ => Self::Hash,
        }
    }

    // The two lines a block opens with: the notice behind `BEGIN`, then the instruction.
    pub fn begin(self, name: &str) -> String {
        let notice = self.wrap(&format!("{BEGIN}{}", template::managed(name)));
        format!("{notice}\n{}", self.wrap(template::INSTRUCTION))
    }

    pub fn end(self, name: &str) -> String {
        self.wrap(&format!("{END}{}", template::managed(name)))
    }

    fn wrap(self, text: &str) -> String {
        match self {
            Self::Html => format!("<!-- {text} -->"),
            Self::Hash => format!("# {text}"),
        }
    }

    // The marker a line is, if it is one: `(is_begin, name)`, in this release's
    // spelling or the one before it.
    pub fn parse(self, line: &str) -> Option<(bool, &str)> {
        let inner = match self {
            Self::Html => line.trim().strip_prefix("<!--")?.strip_suffix("-->")?.trim(),
            Self::Hash => line.trim().strip_prefix('#')?.trim(),
        };
        for (edge, is_begin) in [(BEGIN, true), (END, false)] {
            let managed =
                inner.strip_prefix(edge).and_then(|rest| rest.strip_prefix(template::MANAGED));
            if let Some(name) = managed {
                return Some((is_begin, name.trim()));
            }
        }
        for (edge, is_begin) in [(LEGACY_BEGIN, true), (LEGACY_END, false)] {
            if let Some(name) = inner.strip_prefix(edge) {
                return Some((is_begin, name.trim()));
            }
        }
        None
    }
}

// A marker pair: the line indices of its begin and end markers.
pub struct Span {
    pub name: String,
    pub begin: usize,
    pub end: usize,
}

// Every marker pair of a file, or why they cannot be relied on: a begin without
// its end, an end without a begin, a pair inside another, a name used twice.
pub fn scan(lines: &[&str], style: Style) -> Result<Vec<Span>> {
    let mut spans: Vec<Span> = Vec::new();
    let mut open: Option<(String, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        let Some((is_begin, name)) = style.parse(line) else { continue };
        let at = index + 1;
        match (&open, is_begin) {
            (Some((outer, _)), true) => {
                bail!("line {at}: `{name}` begins inside `{outer}`; blocks never nest")
            }
            (None, true) => {
                if spans.iter().any(|span| span.name == name) {
                    bail!("line {at}: `{name}` begins a second time; a block appears once");
                }
                open = Some((name.to_owned(), index));
            }
            (None, false) => bail!("line {at}: `{name}` ends without having begun"),
            (Some((outer, _)), false) if outer != name => {
                bail!("line {at}: `{name}` ends while `{outer}` is open")
            }
            (Some((_, begin)), false) => {
                spans.push(Span {
                    name: name.to_owned(),
                    begin: *begin,
                    end: index,
                });
                open = None;
            }
        }
    }
    if let Some((name, begin)) = open {
        bail!("line {}: `{name}` begins and never ends", begin + 1);
    }
    Ok(spans)
}

// The file with `body` as the block `name`: the pair named `name`, or `legacy`
// (its name before 0.4.0), is rewritten from its begin marker to its end
// marker, both included, so an old pair is respelled in place; a file with
// neither takes the block at its end after a blank line.
pub fn splice(text: &str, style: Style, name: &str, legacy: &str, body: &str) -> Result<String> {
    let lines: Vec<&str> = text.lines().collect();
    let spans = scan(&lines, style)?;
    let body = body.strip_suffix('\n').unwrap_or(body);
    let (begin, end) = (style.begin(name), style.end(name));
    let mut out: Vec<&str> = Vec::with_capacity(lines.len() + 5);

    if let Some(span) = spans.iter().find(|span| span.name == name || span.name == legacy) {
        out.extend_from_slice(&lines[..span.begin]);
        out.extend(begin.lines());
        out.extend(body.lines());
        out.push(&end);
        out.extend_from_slice(&lines[span.end + 1..]);
    } else {
        out.extend_from_slice(&lines);
        while out.last().is_some_and(|line| line.trim().is_empty()) {
            out.pop();
        }
        if !out.is_empty() {
            out.push("");
        }
        out.extend(begin.lines());
        out.extend(body.lines());
        out.push(&end);
    }

    let mut joined = out.join("\n");
    joined.push('\n');
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIT: &str = "conventions/agents/git.md";

    #[test]
    fn appends_after_existing_text() {
        let out = splice("# Title\n\nintro\n", Style::Html, GIT, "agents/git", "## Git\n\nbody\n")
            .unwrap();
        assert_eq!(
            out,
            "# Title\n\nintro\n\n<!-- BEGIN Managed by augentic/toolkit: conventions/agents/git.md -->\n<!-- Do not edit: run `make conventions-sync`. -->\n## Git\n\nbody\n<!-- END Managed by augentic/toolkit: conventions/agents/git.md -->\n"
        );
    }

    #[test]
    fn replaces_between_markers_wherever_they_sit() {
        let text = "head\n  # BEGIN Managed by augentic/toolkit: x\n  # Do not edit: run `make conventions-sync`.\nold\n  # END Managed by augentic/toolkit: x\ntail\n";
        let out = splice(text, Style::Hash, "x", "x-legacy", "new\n").unwrap();
        assert_eq!(
            out,
            "head\n# BEGIN Managed by augentic/toolkit: x\n# Do not edit: run `make conventions-sync`.\nnew\n# END Managed by augentic/toolkit: x\ntail\n"
        );
    }

    #[test]
    fn respells_a_legacy_pair_in_place() {
        let text = "# Title\n\n<!-- conventions:begin agents/git -->\n## Git\n\nold\n<!-- conventions:end agents/git -->\n\ntail\n";
        let out = splice(text, Style::Html, GIT, "agents/git", "## Git\n\nnew\n").unwrap();
        assert_eq!(
            out,
            "# Title\n\n<!-- BEGIN Managed by augentic/toolkit: conventions/agents/git.md -->\n<!-- Do not edit: run `make conventions-sync`. -->\n## Git\n\nnew\n<!-- END Managed by augentic/toolkit: conventions/agents/git.md -->\n\ntail\n"
        );
        assert_eq!(out.matches("## Git").count(), 1);
    }

    #[test]
    fn a_header_is_not_a_marker() {
        assert!(
            Style::Hash.parse("# Managed by augentic/toolkit: conventions/rustfmt.toml").is_none()
        );
        assert!(Style::Hash.parse("# Do not edit: run `make conventions-sync`.").is_none());
        assert!(Style::Hash.parse("# END of the section").is_none());
        assert_eq!(
            Style::Hash.parse("# END Managed by augentic/toolkit: conventions/gitignore/head"),
            Some((false, "conventions/gitignore/head"))
        );
    }

    #[test]
    fn refuses_nested_unpaired_and_duplicate_markers() {
        let style = Style::Hash;
        let begin = |name: &str| format!("# BEGIN Managed by augentic/toolkit: {name}");
        let end = |name: &str| format!("# END Managed by augentic/toolkit: {name}");
        let nested = [begin("a"), begin("b"), end("b"), end("a")];
        let nested: Vec<&str> = nested.iter().map(String::as_str).collect();
        assert!(scan(&nested, style).is_err());
        assert!(scan(&[begin("a").as_str()], style).is_err());
        assert!(scan(&[end("a").as_str()], style).is_err());
        let twice = [begin("a"), end("a"), begin("a"), end("a")];
        let twice: Vec<&str> = twice.iter().map(String::as_str).collect();
        assert!(scan(&twice, style).is_err());
        assert!(scan(&[begin("a").as_str(), end("b").as_str()], style).is_err());
        assert!(scan(&["# conventions:begin a", "# conventions:end b"], style).is_err());
        assert!(scan(&["# conventions:begin a"], style).is_err());
    }
}
