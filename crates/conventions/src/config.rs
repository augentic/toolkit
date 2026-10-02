use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;

use crate::toolkit::Toolkit;

// The consumer's `conventions.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub name: String,
    #[serde(default)]
    pub targets: Vec<String>,
    #[serde(default)]
    pub wasm_packages: Vec<String>,
    #[serde(default)]
    pub outdated_ignore: Vec<String>,
    #[serde(default)]
    pub guest_clippy: Vec<String>,
    #[serde(default)]
    pub pinned: Vec<String>,
}

// The repository under `--root`, read once: its configuration, its `mise.toml`,
// and the pin every rendered reference carries.
pub struct Consumer {
    pub root: PathBuf,
    pub config: Config,
    pub mise: Option<String>,
    pub pin: String,
}

pub const CONFIG: &str = "conventions.toml";
pub const MISE: &str = "mise.toml";
const INCLUDE: &str = "github.com/augentic/toolkit.git//mise/rust.toml?ref=";

impl Consumer {
    pub fn open(root: &Path, toolkit: &Toolkit) -> Result<Self> {
        let path = root.join(CONFIG);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("{CONFIG} is required at {}", root.display()))?;
        let config: Config =
            toml_edit::de::from_str(&text).with_context(|| format!("parsing {CONFIG}"))?;
        if config.name.is_empty() {
            bail!("{CONFIG}: `name` must not be empty");
        }

        let mise = match std::fs::read_to_string(root.join(MISE)) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).with_context(|| format!("reading {MISE}")),
        };

        // the embedded tree is a release, pinned by mise.toml; a checkout is
        // whatever version it says it is
        let pin = if toolkit.is_embedded() {
            mise.as_deref().and_then(mise_pin).with_context(|| {
                format!("{MISE} does not include {INCLUDE}vX.Y.Z, so there is no pin to write")
            })?
        } else {
            Toolkit::pin()
        };

        Ok(Self {
            root: root.to_path_buf(),
            config,
            mise,
            pin,
        })
    }

    // What the placeholders of a stub or a target path render to.
    pub fn values(&self) -> Values {
        let quoted: Vec<String> = self.config.targets.iter().map(|t| format!("\"{t}\"")).collect();
        let mut scalars = BTreeMap::new();
        scalars.insert("name", self.config.name.clone());
        scalars.insert("version", self.pin.clone());
        scalars.insert("targets", self.config.targets.join(","));
        scalars.insert("targets-toml", quoted.join(", "));
        scalars.insert("wasm-packages", self.config.wasm_packages.join(" "));
        scalars.insert("outdated-ignore", self.config.outdated_ignore.join(","));
        let mut lists = BTreeMap::new();
        lists.insert("guest-clippy", self.config.guest_clippy.clone());
        Values { scalars, lists }
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

pub struct Values {
    pub scalars: BTreeMap<&'static str, String>,
    pub lists: BTreeMap<&'static str, Vec<String>>,
}

impl Values {
    // Every target a manifest path stands for: itself, or one per entry of the
    // list-valued key it names.
    pub fn targets(&self, target: &str) -> Result<Vec<String>> {
        let Some(key) = crate::template::sole_placeholder(target) else {
            return Ok(vec![target.to_owned()]);
        };
        match self.lists.get(key) {
            Some(list) => Ok(list.clone()),
            None => bail!("manifest target {target} names no list-valued key of {CONFIG}"),
        }
    }
}
