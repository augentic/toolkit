use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use include_dir::{Dir, include_dir};

use crate::manifest::Manifest;

static EMBEDDED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../conventions");

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Where the conventions come from: the tree compiled into this binary, or the
// `conventions/` directory of a checkout for development and the pre-tag dry run.
pub enum Toolkit {
    Embedded,
    Checkout(PathBuf),
}

impl Toolkit {
    pub const fn embedded() -> Self {
        Self::Embedded
    }

    pub fn checkout(dir: &Path) -> Result<Self> {
        let tree = dir.join("conventions");
        if !tree.join("manifest.toml").is_file() {
            bail!("{} is not a toolkit checkout: no conventions/manifest.toml", dir.display());
        }
        Ok(Self::Checkout(tree))
    }

    pub fn manifest(&self) -> Result<Manifest> {
        let text = self.read("manifest.toml")?;
        toml_edit::de::from_str(&text).context("parsing conventions/manifest.toml")
    }

    pub fn read(&self, source: &str) -> Result<String> {
        match self {
            Self::Embedded => EMBEDDED
                .get_file(source)
                .and_then(|file| file.contents_utf8())
                .map(str::to_owned)
                .with_context(|| format!("conventions/{source} is not in the embedded tree")),
            Self::Checkout(tree) => std::fs::read_to_string(tree.join(source))
                .with_context(|| format!("reading {}", tree.join(source).display())),
        }
    }

    // The tag the rendered stubs and the pin rewrites name: the version this
    // program was built from, since a release embeds its own tree.
    pub fn pin() -> String {
        format!("v{VERSION}")
    }

    pub const fn is_embedded(&self) -> bool {
        matches!(self, Self::Embedded)
    }
}
