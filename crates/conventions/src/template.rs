use crate::manifest::Header;

// The managed notice is two lines: the toolkit and the source after `MANAGED`,
// then `INSTRUCTION`. Whole files open with it; a block's markers carry it
// behind `BEGIN` and `END`.
pub const MANAGED: &str = "Managed by augentic/toolkit: ";
pub const INSTRUCTION: &str = "Do not edit: run `make conventions-sync`.";
// The one-line header 0.3.0 wrote on a rendered file, read for this release so
// a caller workflow or `rust-toolchain.toml` loses it at the first sync.
const LEGACY: &str = "# Managed by augentic/toolkit from ";

pub fn managed(from: &str) -> String {
    format!("{MANAGED}{from}")
}

// The notice a managed whole file opens with, a blank line after it.
pub fn header(header: Header, from: &str) -> String {
    match header {
        Header::Hash => format!("# {}\n# {INSTRUCTION}\n\n", managed(from)),
        Header::Html => format!("<!-- {} -->\n<!-- {INSTRUCTION} -->\n\n", managed(from)),
        Header::None => String::new(),
    }
}

// The text without a 0.3.0 header on its first line and the blank line after it.
pub fn legacy_header(text: &str) -> &str {
    if !text.starts_with(LEGACY) {
        return text;
    }
    let rest = text.find('\n').map_or("", |at| &text[at + 1..]);
    rest.strip_prefix('\n').unwrap_or(rest)
}
