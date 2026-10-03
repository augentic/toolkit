use std::path::Path;

use anyhow::{Context as _, Result};
use similar::TextDiff;

use crate::consumer::Consumer;
use crate::manifest::block_name;
use crate::marker::{self, Style};
use crate::toolkit::Toolkit;
use crate::{pin, rules, table, template};

// One managed file: what the repository has, what the toolkit says, and the
// notes to print beside it (the repository's own keys in a shared table).
pub struct Managed {
    pub path: String,
    pub current: Option<String>,
    pub desired: String,
    pub notes: Vec<String>,
}

impl Managed {
    fn differs(&self) -> bool {
        self.current.as_deref() != Some(self.desired.as_str())
    }
}

// Every managed file resolved against the repository, plus the files that
// could not be resolved and why: markers that cannot be relied on, a table the
// file cannot hold.
pub struct Plan {
    pub files: Vec<Managed>,
    pub problems: Vec<String>,
}

impl Plan {
    pub fn build(toolkit: &Toolkit, consumer: &Consumer) -> Result<Self> {
        let manifest = toolkit.manifest()?;
        let mut plan = Self {
            files: Vec::new(),
            problems: Vec::new(),
        };

        for whole in &manifest.whole {
            let body = toolkit.read(&whole.source)?;
            let desired =
                template::header(whole.header, &format!("conventions/{}", whole.source)) + &body;
            plan.push(consumer, &whole.target, desired, Vec::new())?;
        }

        for blocks in &manifest.block {
            let style = Style::of(Path::new(&blocks.target));
            let mut text = read(&consumer.root, &blocks.target)?
                .or_else(|| blocks.preamble.clone())
                .unwrap_or_default();
            let mut failed = None;
            for source in &blocks.sources {
                let body = toolkit.read(source)?;
                let name = format!("conventions/{source}");
                match marker::splice(&text, style, &name, block_name(source), &body) {
                    Ok(next) => text = next,
                    Err(error) => {
                        failed = Some(error);
                        break;
                    }
                }
            }
            match failed {
                Some(error) => plan.problems.push(format!("{}: {error:#}", blocks.target)),
                None => plan.push(consumer, &blocks.target, text, Vec::new())?,
            }
        }

        for tables in &manifest.table {
            let bodies = tables
                .sources
                .iter()
                .map(|source| toolkit.read(source))
                .collect::<Result<Vec<_>>>()?;
            let mut text = read(&consumer.root, &tables.target)?.unwrap_or_default();
            match settle(&mut text, &bodies, &tables.retired) {
                Ok(locals) => {
                    let notes =
                        locals.into_iter().map(|local| note(&tables.target, &local)).collect();
                    plan.push(consumer, &tables.target, text, notes)?;
                }
                Err(error) => plan.problems.push(format!("{}: {error:#}", tables.target)),
            }
        }

        // every workflow is the repository's and carries the pin alone
        for path in workflows(&consumer.root)? {
            let Some(current) = read(&consumer.root, &path)? else { continue };
            let desired = pin::rewrite(template::legacy_header(&current), &consumer.pin);
            plan.files.push(Managed {
                path,
                current: Some(current),
                desired,
                notes: Vec::new(),
            });
        }

        Ok(plan)
    }

    fn push(
        &mut self, consumer: &Consumer, path: &str, desired: String, notes: Vec<String>,
    ) -> Result<()> {
        let current = read(&consumer.root, path)?;
        self.files.push(Managed {
            path: path.to_owned(),
            current,
            desired,
            notes,
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
        self.print_notes();
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
        self.print_notes();

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

    fn print_notes(&self) {
        for note in self.files.iter().flat_map(|file| &file.notes) {
            println!("{note}");
        }
    }
}

// The text with every body's keys held and the retired paths gone, and the
// repository's own keys in the tables the bodies set.
fn settle(text: &mut String, bodies: &[String], retired: &[String]) -> Result<Vec<table::Local>> {
    *text = template::legacy_header(text).to_owned();
    for body in bodies {
        *text = table::set(text, body)?;
    }
    *text = table::retire(text, retired)?;
    let mut locals = Vec::new();
    for body in bodies {
        locals.extend(table::local(text, body)?);
    }
    Ok(locals)
}

fn note(target: &str, local: &table::Local) -> String {
    let keys = local.keys.join(", ");
    let at = local.header.as_ref().map_or(String::new(), |header| format!(" {header}"));
    format!("{target}{at}: repository keys {keys}")
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
