use serde::Deserialize;

// The shape of `conventions/manifest.toml`: one list per mode. A `source` is
// a path under `conventions/`, a `target` a path under the consumer root that
// may carry `{{key}}` placeholders.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default)]
    pub whole: Vec<Whole>,
    #[serde(default)]
    pub block: Vec<Blocks>,
    #[serde(default)]
    pub table: Vec<Tables>,
    #[serde(default)]
    pub stub: Vec<Stub>,
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

// `markers = false` is for a file another tool rewrites in its own layout
// (`cargo vet` owns `supply-chain/config.toml`): the keys the block sets must
// hold its values, and nothing else about the file is held.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tables {
    pub target: String,
    pub sources: Vec<String>,
    #[serde(default = "yes")]
    pub markers: bool,
}

const fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stub {
    pub source: String,
    pub target: String,
}

// A block's name is its source path without the extension.
pub fn block_name(source: &str) -> &str {
    match source.rsplit_once('.') {
        Some((stem, extension)) if !extension.contains('/') => stem,
        _ => source,
    }
}
