use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

use crate::toolkit::Toolkit;

// The repository under `--root`: the pin every rendered reference carries,
// which is the version of this program since a release embeds its own tree,
// and the pin its `mise.toml` names when the file includes the toolkit's tasks.
pub struct Consumer {
    pub root: PathBuf,
    pub pin: String,
    pub mise_pin: Option<String>,
}

pub const MISE: &str = "mise.toml";
const INCLUDE: &str = "github.com/augentic/toolkit.git//mise/rust.toml?ref=";

impl Consumer {
    pub fn open(root: &Path) -> Result<Self> {
        let mise_pin = match std::fs::read_to_string(root.join(MISE)) {
            Ok(text) => mise_pin(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).with_context(|| format!("reading {MISE}")),
        };
        Ok(Self {
            root: root.to_path_buf(),
            pin: Toolkit::pin(),
            mise_pin,
        })
    }
}

// The `vX.Y.Z` after `?ref=` in the toolkit include line, if the file has one.
fn mise_pin(mise: &str) -> Option<String> {
    let start = mise.find(INCLUDE)? + INCLUDE.len();
    let tail = &mise[start..];
    let end = tail.find(|c: char| c == '"' || c == '\'' || c.is_whitespace()).unwrap_or(tail.len());
    let pin = &tail[..end];
    (!pin.is_empty()).then(|| pin.to_owned())
}
