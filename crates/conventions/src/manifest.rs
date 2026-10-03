use serde::Deserialize;

// The shape of `conventions/manifest.toml`: one list per mode. A `source` is
// a path under `conventions/`, a `target` a path under the consumer root.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub whole: Vec<Whole>,
    #[serde(default)]
    pub block: Vec<Blocks>,
    #[serde(default)]
    pub table: Vec<Tables>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Whole {
    pub source: String,
    pub target: String,
    pub header: Header,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Header {
    Hash,
    Html,
    None,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blocks {
    pub target: String,
    pub preamble: Option<String>,
    pub sources: Vec<String>,
}

// `retired` lists the dotted keys the sources once set, removed from the
// target when present.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tables {
    pub target: String,
    pub sources: Vec<String>,
    #[serde(default)]
    pub retired: Vec<String>,
}

// A block's name before 0.4.0, its source path without the extension, which
// `marker::splice` reads to respell an old pair in place.
pub fn block_name(source: &str) -> &str {
    match source.rsplit_once('.') {
        Some((stem, extension)) if !extension.contains('/') => stem,
        _ => source,
    }
}
