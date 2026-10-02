use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context as _, Result};
use similar::TextDiff;

use crate::config::Consumer;
use crate::manifest::{Header, block_name};
use crate::marker::{self, Style};
use crate::toolkit::Toolkit;
use crate::{pin, rules, table, template};

// One managed file: what the repository has and what the toolkit says.
pub struct Managed {
    pub path: String,
    pub current: Option<String>,
    pub desired: String,
}

impl Managed {
    fn differs(&self) -> bool {
        self.current.as_deref() != Some(self.desired.as_str())
    }
}

// Every managed file resolved against the repository, plus the files that
// could not be resolved and why: markers that cannot be relied on, a block
// that leaves a TOML file invalid, a template that does not render.
pub struct Plan {
    pub files: Vec<Managed>,
    pub problems: Vec<String>,
}

impl Plan {
    pub fn build(toolkit: &Toolkit, consumer: &Consumer) -> Result<Self> {
        let manifest = toolkit.manifest()?;
        let values = consumer.values();
        let mut plan = Self {
            files: Vec::new(),
            problems: Vec::new(),
        };
        let mut stubs = BTreeSet::new();

        for whole in &manifest.whole {
            let body = toolkit.read(&whole.source)?;
            let desired =
                template::header(whole.header, &format!("conventions/{}", whole.source)) + &body;
            plan.push(consumer, &whole.target, desired)?;
        }

        for blocks in &manifest.block {
            let style = Style::of(Path::new(&blocks.target));
            let mut text = match read(&consumer.root, &blocks.target)? {
                Some(text) => text,
                None => match &blocks.preamble {
                    Some(preamble) => template::render(preamble, &values)?,
                    None => String::new(),
                },
            };
            let mut failed = None;
            for source in &blocks.sources {
                let body = toolkit.read(source)?;
                match marker::splice(&text, style, block_name(source), &body) {
                    Ok(next) => text = next,
                    Err(error) => {
                        failed = Some(error);
                        break;
                    }
                }
            }
            match failed {
                Some(error) => plan.problems.push(format!("{}: {error:#}", blocks.target)),
                None => plan.push(consumer, &blocks.target, text)?,
            }
        }

        for tables in &manifest.table {
            for target in values.targets(&tables.target)? {
                let mut text = read(&consumer.root, &target)?.unwrap_or_default();
                let mut failed = None;
                for source in &tables.sources {
                    let body = toolkit.read(source)?;
                    let merged = if tables.markers {
                        table::merge(&text, block_name(source), &body)
                    } else {
                        table::set(&text, &body)
                    };
                    match merged {
                        Ok(next) => text = next,
                        Err(error) => {
                            failed = Some(error);
                            break;
                        }
                    }
                }
                match failed {
                    Some(error) => plan.problems.push(format!("{target}: {error:#}")),
                    None => plan.push(consumer, &target, text)?,
                }
            }
        }

        for stub in &manifest.stub {
            let rendered = template::render(&toolkit.read(&stub.source)?, &values)
                .with_context(|| format!("rendering conventions/{}", stub.source))?;
            let from = format!("conventions/{} and {}", stub.source, crate::config::CONFIG);
            plan.push(consumer, &stub.target, template::header(Header::Hash, &from) + &rendered)?;
            stubs.insert(stub.target.clone());
        }

        // every other workflow, and each file the configuration names, carries
        // the pin alone
        let mut pinned: Vec<String> =
            workflows(&consumer.root)?.into_iter().filter(|p| !stubs.contains(p)).collect();
        pinned.extend(consumer.config.pinned.iter().cloned());
        for path in pinned {
            match read(&consumer.root, &path)? {
                Some(current) => {
                    let desired = pin::rewrite(&current, &consumer.pin);
                    plan.files.push(Managed {
                        path,
                        current: Some(current),
                        desired,
                    });
                }
                None => plan
                    .problems
                    .push(format!("{path}: named by `pinned` but not in the repository")),
            }
        }

        Ok(plan)
    }

    fn push(&mut self, consumer: &Consumer, path: &str, desired: String) -> Result<()> {
        let current = read(&consumer.root, path)?;
        self.files.push(Managed {
            path: path.to_owned(),
            current,
            desired,
        });
        Ok(())
    }

    // Writes every file that differs. False when a file could not be planned.
    pub fn sync(&self, root: &Path) -> Result<bool> {
        for file in self.files.iter().filter(|file| file.differs()) {
            let path = root.join(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::write(&path, &file.desired)
                .with_context(|| format!("writing {}", path.display()))?;
            println!("{} {}", if file.current.is_some() { "wrote" } else { "created" }, file.path);
        }
        for problem in &self.problems {
            eprintln!("unresolved: {problem}");
        }
        Ok(self.problems.is_empty())
    }

    // Reports every difference and every broken rule. True when there is none.
    pub fn check(&self, toolkit: &Toolkit, consumer: &Consumer) -> bool {
        let mut failures = 0;
        for file in self.files.iter().filter(|file| file.differs()) {
            failures += 1;
            match &file.current {
                None => println!("missing: {}", file.path),
                Some(current) => {
                    let diff = TextDiff::from_lines(current.as_str(), file.desired.as_str());
                    print!(
                        "{}",
                        diff.unified_diff().header(
                            &format!("{} (repository)", file.path),
                            &format!("{} (conventions)", file.path)
                        )
                    );
                }
            }
        }
        for problem in &self.problems {
            failures += 1;
            println!("unresolved: {problem}");
        }
        for failure in rules::failures(toolkit, consumer) {
            failures += 1;
            println!("rule: {failure}");
        }

        if failures == 0 {
            println!("conventions: {} managed files match", self.files.len());
            true
        } else {
            println!(
                "conventions: {failures} problem(s); run `make conventions-sync` for the files, fix the rest by hand"
            );
            false
        }
    }
}

fn read(root: &Path, path: &str) -> Result<Option<String>> {
    match std::fs::read_to_string(root.join(path)) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {path}")),
    }
}

// The repository's workflow files, as `.github/workflows/<name>`.
fn workflows(root: &Path) -> Result<Vec<String>> {
    let dir = root.join(".github/workflows");
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(error) => return Err(error).with_context(|| format!("listing {}", dir.display())),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let extension = Path::new(name).extension().and_then(|extension| extension.to_str());
        if entry.file_type()?.is_file() && matches!(extension, Some("yaml" | "yml")) {
            out.push(format!(".github/workflows/{name}"));
        }
    }
    out.sort();
    Ok(out)
}
