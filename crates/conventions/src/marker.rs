use std::path::Path;

use anyhow::{Result, bail};

// The comment syntax a marker pair is written in: HTML comments in Markdown,
// `#` comments everywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Html,
    Hash,
}

const BEGIN: &str = "conventions:begin ";
const END: &str = "conventions:end ";

impl Style {
    pub fn of(target: &Path) -> Self {
        match target.extension().and_then(|e| e.to_str()) {
            Some("md") => Self::Html,
            _ => Self::Hash,
        }
    }

    pub fn begin(self, name: &str) -> String {
        self.wrap(&format!("{BEGIN}{name}"))
    }

    pub fn end(self, name: &str) -> String {
        self.wrap(&format!("{END}{name}"))
    }

    fn wrap(self, text: &str) -> String {
        match self {
            Self::Html => format!("<!-- {text} -->"),
            Self::Hash => format!("# {text}"),
        }
    }

    // The marker a line is, if it is one: `(is_begin, name)`.
    fn parse(self, line: &str) -> Option<(bool, &str)> {
        let inner = match self {
            Self::Html => line.trim().strip_prefix("<!--")?.strip_suffix("-->")?.trim(),
            Self::Hash => line.trim().strip_prefix('#')?.trim(),
        };
        if let Some(name) = inner.strip_prefix(BEGIN) {
            return Some((true, name.trim()));
        }
        if let Some(name) = inner.strip_prefix(END) {
            return Some((false, name.trim()));
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

// The file with `body` between the markers of `name`: replacing what was there,
// or appended as a new pair after a blank line when the file has none.
pub fn splice(text: &str, style: Style, name: &str, body: &str) -> Result<String> {
    let lines: Vec<&str> = text.lines().collect();
    let spans = scan(&lines, style)?;
    let body = body.strip_suffix('\n').unwrap_or(body);
    let mut out: Vec<&str> = Vec::with_capacity(lines.len() + 4);
    let (begin, end) = (style.begin(name), style.end(name));

    if let Some(span) = spans.iter().find(|span| span.name == name) {
        out.extend_from_slice(&lines[..=span.begin]);
        out.extend(body.lines());
        out.extend_from_slice(&lines[span.end..]);
    } else {
        out.extend_from_slice(&lines);
        while out.last().is_some_and(|line| line.trim().is_empty()) {
            out.pop();
        }
        if !out.is_empty() {
            out.push("");
        }
        out.push(&begin);
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

    #[test]
    fn appends_after_existing_text() {
        let out = splice("# Title\n\nintro\n", Style::Html, "a/b", "## B\n\nbody\n").unwrap();
        assert_eq!(
            out,
            "# Title\n\nintro\n\n<!-- conventions:begin a/b -->\n## B\n\nbody\n<!-- conventions:end a/b -->\n"
        );
    }

    #[test]
    fn replaces_between_markers_wherever_they_sit() {
        let text = "head\n  # conventions:begin x\nold\n  # conventions:end x\ntail\n";
        let out = splice(text, Style::Hash, "x", "new\n").unwrap();
        assert_eq!(out, "head\n  # conventions:begin x\nnew\n  # conventions:end x\ntail\n");
    }

    #[test]
    fn refuses_nested_unpaired_and_duplicate_markers() {
        let style = Style::Hash;
        let nested = [
            "# conventions:begin a",
            "# conventions:begin b",
            "# conventions:end b",
            "# conventions:end a",
        ];
        assert!(scan(&nested, style).is_err());
        assert!(scan(&["# conventions:begin a"], style).is_err());
        assert!(scan(&["# conventions:end a"], style).is_err());
        let twice = [
            "# conventions:begin a",
            "# conventions:end a",
            "# conventions:begin a",
            "# conventions:end a",
        ];
        assert!(scan(&twice, style).is_err());
        assert!(scan(&["# conventions:begin a", "# conventions:end b"], style).is_err());
    }
}
