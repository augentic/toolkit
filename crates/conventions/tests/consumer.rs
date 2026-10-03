//! Drives the `conventions` binary over a fixture consumer with the
//! repository's own `conventions/` tree.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;
use toml_edit::DocumentMut;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTRUCTION: &str = "Do not edit: run `make conventions-sync`.";
const CLIPPY: &str = "doc-valid-idents = [\"GitHub\"]\n";
const TOOLCHAIN: &str = "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\", \"rust-src\", \"rustfmt\"]\ntargets = [\"wasm32-wasip2\"]\n";
const CI_CALLER: &str = "name: CI\non:\n  pull_request:\njobs:\n  ci:\n    uses: augentic/toolkit/.github/workflows/ci.yaml@v0.3.0\n    secrets: inherit\n    with:\n      targets: wasm32-wasip2\n";

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    // A consumer that already carries unmarked copies of what the blocks
    // bring, local keys in the shared tables, stale pins, and the one-line
    // header 0.3.0 rendered onto `rust-toolchain.toml` and its `ci.yaml`.
    fn new() -> Self {
        let fixture = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fixture.write(
            "mise.toml",
            &format!(
                "[task_config]\nincludes = [\"git::https://github.com/augentic/toolkit.git//mise/rust.toml?ref=v{VERSION}\"]\n\n[env]\nWASM32_PACKAGES = \"guest\"\n"
            ),
        );
        fixture.write(
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.lints.rust]\nunsafe_code = \"deny\"\nelided_lifetimes_in_paths = \"warn\"\n\n[workspace.lints.clippy]\nall = { level = \"warn\", priority = -1 }\nmissing_errors_doc = \"allow\"\n\n[profile.release]\nlto = true\n",
        );
        fixture.write(
            "deny.toml",
            "[graph]\nall-features = true\n\n[advisories]\nunmaintained = 'workspace'\nignore = []\n\n[licenses]\nallow = [\"MIT\"]\nexceptions = [{ allow = [\"Unicode-DFS-2016\"], crate = \"unicode_names2\" }]\n\n[licenses.private]\nignore = true\n",
        );
        fixture.write(
            "supply-chain/config.toml",
            "[cargo-vet]\nversion = \"0.10\"\n\n[imports.mozilla]\nurl = \"https://old\"\n\n[[exemptions.x]]\nversion = \"1.0.0\"\ncriteria = \"safe-to-deploy\"\n",
        );
        fixture.write("clippy.toml", CLIPPY);
        fixture.write("AGENTS.md", "# Fixture\n\nWhat this repository is.\n");
        fixture.write(".gitignore", "target/\nfoo/\n");
        fixture.write(
            "rust-toolchain.toml",
            &(legacy_header("conventions/rust-toolchain.toml and conventions.toml") + TOOLCHAIN),
        );
        fixture.write(
            ".github/workflows/ci.yaml",
            &(legacy_header("conventions/workflows/ci.yaml and conventions.toml") + CI_CALLER),
        );
        fixture.write(
            ".github/workflows/publish.yaml",
            "name: Publish\njobs:\n  publish:\n    uses: augentic/toolkit/.github/workflows/publish.yaml@v0.1.0\n",
        );
        fixture
    }

    // A consumer as 0.3.0 left it: every shared table between hash markers
    // with the local keys after the end marker, the guest deny-list between
    // its own pair, and every block between the `conventions:begin` /
    // `conventions:end` pair of that release.
    fn v0_3_0() -> Self {
        let fixture = Self::new();
        let rust = legacy_table("lints/rust").replacen(
            "[workspace.lints.rust]\n",
            "[workspace.lints.rust]\n# https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html\n",
            1,
        );
        fixture.write(
            "Cargo.toml",
            &format!(
                "[workspace]\nmembers = [\"crates/*\"]\n\n{rust}elided_lifetimes_in_paths = \"warn\"\n\n{}missing_errors_doc = \"allow\"\n\n[profile.release]\nlto = true\n",
                legacy_table("lints/clippy"),
            ),
        );
        fixture.write(
            "deny.toml",
            &format!(
                "[graph]\nall-features = true\n\n{}ignore = []\n\n{}exceptions = [{{ allow = [\"Unicode-DFS-2016\"], crate = \"unicode_names2\" }}]\n\n[licenses.private]\nignore = true\n",
                legacy_table("deny/advisories"),
                legacy_table("deny/licenses"),
            ),
        );
        fixture.write(
            "clippy.toml",
            &format!(
                "{CLIPPY}\n# conventions:begin clippy/guest-deny-list\ndisallowed-methods = [\n  {{ path = \"std::env::args\", reason = \"Omnia command routing owns argv\" }},\n]\n# conventions:end clippy/guest-deny-list\n"
            ),
        );
        fixture.write(
            "AGENTS.md",
            "# Fixture\n\nWhat this repository is.\n\n<!-- conventions:begin agents/git -->\n## Git\n\nThe 0.3.0 text.\n<!-- conventions:end agents/git -->\n\n## Map\n\nThe repository's own section.\n\n<!-- conventions:begin agents/commands -->\n## Commands\n\nThe 0.3.0 text.\n<!-- conventions:end agents/commands -->\n",
        );
        fixture.write(
            ".gitignore",
            "# conventions:begin gitignore/head\ntarget/\n.env\n# conventions:end gitignore/head\nfoo/\n",
        );
        fixture.write(
            ".github/dependabot.yml",
            "version: 2\nupdates:\n\n# conventions:begin dependabot/actions\n  - package-ecosystem: github-actions\n    directory: /\n# conventions:end dependabot/actions\n",
        );
        fixture
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path(relative)).unwrap()
    }

    fn edit(&self, relative: &str, edit: impl FnOnce(String) -> String) {
        self.write(relative, &edit(self.read(relative)));
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conventions"))
            .args(["--root", self.dir.path().to_str().unwrap()])
            .args(args)
            .output()
            .unwrap()
    }

    fn checkout(&self, command: &str) -> Output {
        self.run(&[command, "--toolkit", toolkit().to_str().unwrap()])
    }

    // Every file under the root that carries a 0.3.0 marker or header line.
    fn legacy_left(&self) -> Vec<String> {
        let mut found = Vec::new();
        let mut pending = vec![self.dir.path().to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    pending.push(path);
                } else if std::fs::read_to_string(&path).is_ok_and(|text| {
                    text.contains("conventions:begin")
                        || text.contains("conventions:end")
                        || text.contains("Managed by augentic/toolkit from")
                }) {
                    found.push(path.strip_prefix(self.dir.path()).unwrap().display().to_string());
                }
            }
        }
        found.sort();
        found
    }
}

fn toolkit() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// The one-line header 0.3.0 wrote on a rendered file.
fn legacy_header(from: &str) -> String {
    format!(
        "# Managed by augentic/toolkit from {from}. Do not edit: run `make conventions-sync`.\n\n"
    )
}

// A shared table as 0.3.0 spliced it: the source between its marker pair.
fn legacy_table(name: &str) -> String {
    let body = std::fs::read_to_string(toolkit().join(format!("conventions/{name}.toml"))).unwrap();
    format!("# conventions:begin {name}\n{body}# conventions:end {name}\n")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn hash_header(from: &str) -> String {
    format!("# Managed by augentic/toolkit: {from}\n# {INSTRUCTION}\n\n")
}

fn html_header(from: &str) -> String {
    format!("<!-- Managed by augentic/toolkit: {from} -->\n<!-- {INSTRUCTION} -->\n\n")
}

fn html_begin(source: &str) -> String {
    format!(
        "<!-- BEGIN Managed by augentic/toolkit: conventions/{source} -->\n<!-- {INSTRUCTION} -->\n"
    )
}

fn html_end(source: &str) -> String {
    format!("<!-- END Managed by augentic/toolkit: conventions/{source} -->\n")
}

fn hash_begin(source: &str) -> String {
    format!("# BEGIN Managed by augentic/toolkit: conventions/{source}\n# {INSTRUCTION}\n")
}

fn hash_end(source: &str) -> String {
    format!("# END Managed by augentic/toolkit: conventions/{source}\n")
}

fn item<'a>(doc: &'a DocumentMut, path: &str) -> &'a toml_edit::Item {
    let mut item = doc.as_item();
    for key in path.split('.') {
        item = item.get(key).unwrap_or_else(|| panic!("{path} not in:\n{doc}"));
    }
    item
}

// The value at a dotted path of a TOML text, rendered without its decor.
fn value(text: &str, path: &str) -> String {
    let doc: DocumentMut = text.parse().unwrap_or_else(|e| panic!("{e}\n{text}"));
    let mut value =
        item(&doc, path).as_value().unwrap_or_else(|| panic!("{path} is not a value")).clone();
    value.decor_mut().clear();
    value.to_string()
}

// The keys of a table of a TOML text, in file order.
fn keys(text: &str, path: &str) -> Vec<String> {
    let doc: DocumentMut = text.parse().unwrap_or_else(|e| panic!("{e}\n{text}"));
    item(&doc, path).as_table().unwrap().iter().map(|(key, _)| key.to_owned()).collect()
}

// Whole files open with the notice; blocks sit after the repository's text
// under the new markers, and a missing file is created.
fn assert_notices_and_blocks(fixture: &Fixture) {
    assert!(fixture.read("rustfmt.toml").starts_with(&hash_header("conventions/rustfmt.toml")));
    assert!(fixture.read("GOVERNANCE.md").starts_with(&html_header("conventions/GOVERNANCE.md")));
    assert!(fixture.read("CODE_OF_CONDUCT.md").contains("## Community Code of Conduct"));
    assert!(!fixture.read("LICENSE-MIT").contains("Managed by"));

    let agents = fixture.read("AGENTS.md");
    assert!(agents.starts_with(&format!(
        "# Fixture\n\nWhat this repository is.\n\n{}## Git\n",
        html_begin("agents/git.md")
    )));
    assert!(agents.ends_with(&html_end("agents/commands.md")));
    assert_eq!(agents.matches("<!-- BEGIN Managed by augentic/toolkit: ").count(), 5);
    assert_eq!(agents.matches("<!-- END Managed by augentic/toolkit: ").count(), 5);
    assert!(
        fixture
            .read("CONTRIBUTING.md")
            .starts_with(&format!("# Contributing\n\n{}", html_begin("contributing/dco.md")))
    );
    let gitignore = fixture.read(".gitignore");
    assert!(
        gitignore
            .starts_with(&format!("target/\nfoo/\n\n{}target/\n", hash_begin("gitignore/head")))
    );
    assert!(gitignore.ends_with(&hash_end("gitignore/head")));
    assert!(fixture.read(".github/dependabot.yml").starts_with(&format!(
        "version: 2\nupdates:\n\n{}  - package-ecosystem: github-actions\n",
        hash_begin("dependabot/actions.yml")
    )));
}

// Shared keys hold the tree's values, a key the file lacked arrives with its
// comment, and the repository's keys stay in place.
fn assert_tables(fixture: &Fixture) {
    let cargo = fixture.read("Cargo.toml");
    assert_eq!(value(&cargo, "workspace.lints.rust.missing_docs"), "\"warn\"");
    assert_eq!(value(&cargo, "workspace.lints.rust.unsafe_code"), "\"deny\"");
    assert_eq!(
        keys(&cargo, "workspace.lints.rust"),
        [
            "missing_docs",
            "trivial_numeric_casts",
            "unsafe_code",
            "unsafe_op_in_unsafe_fn",
            "unused_extern_crates",
            "elided_lifetimes_in_paths"
        ]
    );
    assert!(cargo.contains("[workspace.lints.rust]\nmissing_docs = \"warn\"\n"), "{cargo}");
    assert!(
        cargo.contains(
            "pedantic = { level = \"warn\", priority = -1 }\n\n# Cherry-picked lints from the `restriction` group\nas_pointer_underscore = \"warn\"\n"
        ),
        "{cargo}"
    );
    assert_eq!(keys(&cargo, "workspace.lints.clippy").last().unwrap(), "missing_errors_doc");
    assert_eq!(value(&cargo, "profile.release.lto"), "true");
    assert_eq!(cargo.matches("unsafe_code").count(), 1);
    assert!(!cargo.contains("Managed by"));

    let deny = fixture.read("deny.toml");
    assert_eq!(value(&deny, "advisories.unmaintained"), "\"workspace\"");
    assert_eq!(value(&deny, "advisories.ignore"), "[]");
    assert!(value(&deny, "licenses.allow").contains("\"BSL-1.0\""));
    assert!(value(&deny, "licenses.exceptions").contains("unicode_names2"));
    assert_eq!(value(&deny, "licenses.private.ignore"), "true");
    assert_eq!(value(&deny, "graph.all-features"), "true");

    let vet = fixture.read("supply-chain/config.toml");
    assert_eq!(vet.matches("[imports.mozilla]").count(), 1);
    assert!(!vet.contains("https://old"));
    assert!(value(&vet, "imports.augentic.url").contains("augentic/toolkit"));
    assert!(vet.contains("[[exemptions.x]]"));

    assert_eq!(fixture.read("rust-toolchain.toml"), TOOLCHAIN);
    assert_eq!(fixture.read("clippy.toml"), CLIPPY);
}

// A caller is the repository's: the 0.3.0 header goes, the pin is rewritten,
// and nothing else moves.
fn assert_callers(fixture: &Fixture) {
    let ci = fixture.read(".github/workflows/ci.yaml");
    assert_eq!(ci, CI_CALLER.replace("@v0.3.0", &format!("@v{VERSION}")));
    assert!(
        fixture
            .read(".github/workflows/publish.yaml")
            .contains(&format!("publish.yaml@v{VERSION}\n"))
    );
}

#[test]
fn sync_then_check() {
    let fixture = Fixture::new();
    let sync = fixture.checkout("sync");
    assert!(sync.status.success(), "{}", stderr(&sync));
    assert_eq!(fixture.legacy_left(), Vec::<String>::new());
    assert_notices_and_blocks(&fixture);
    assert_tables(&fixture);
    assert_callers(&fixture);

    // check passes and names the repository's keys in each shared table
    let check = fixture.checkout("check");
    let out = stdout(&check);
    assert!(check.status.success(), "{out}");
    assert!(out.contains("managed files match"));
    for note in [
        "Cargo.toml [workspace.lints.rust]: repository keys elided_lifetimes_in_paths\n",
        "Cargo.toml [workspace.lints.clippy]: repository keys missing_errors_doc\n",
        "deny.toml [advisories]: repository keys ignore\n",
        "deny.toml [licenses]: repository keys exceptions\n",
        "rust-toolchain.toml [toolchain]: repository keys targets\n",
    ] {
        assert!(out.contains(note), "expected `{note}` in\n{out}");
    }
    assert!(!out.contains("supply-chain/config.toml"), "{out}");
    assert!(!out.contains("clippy.toml"), "{out}");

    let again = fixture.checkout("sync");
    let out = stdout(&again);
    assert!(again.status.success());
    assert!(
        !out.contains("wrote") && !out.contains("created"),
        "a second sync writes nothing:\n{out}"
    );
}

#[test]
fn v0_3_0_consumer_migrates() {
    let fixture = Fixture::v0_3_0();
    assert_eq!(fixture.legacy_left().len(), 8);
    let sync = fixture.checkout("sync");
    assert!(sync.status.success(), "{}", stderr(&sync));
    // the deny-list is the repository's now, its old pair included
    assert_eq!(fixture.legacy_left(), ["clippy.toml"]);

    // each block once, respelled where it sat, the repository's text around it
    let agents = fixture.read("AGENTS.md");
    assert!(agents.starts_with(&format!(
        "# Fixture\n\nWhat this repository is.\n\n{}## Git\n",
        html_begin("agents/git.md")
    )));
    assert!(agents.contains(&format!(
        "{}\n## Map\n\nThe repository's own section.\n\n{}",
        html_end("agents/git.md"),
        html_begin("agents/commands.md")
    )));
    assert_eq!(agents.matches("## Git\n").count(), 1);
    assert_eq!(agents.matches("## Commands\n").count(), 1);
    assert!(!agents.contains("The 0.3.0 text."));
    assert_eq!(agents.matches("<!-- BEGIN Managed by augentic/toolkit: ").count(), 5);
    assert_eq!(agents.matches("<!-- END Managed by augentic/toolkit: ").count(), 5);
    let gitignore = fixture.read(".gitignore");
    assert!(gitignore.starts_with(&format!("{}target/\n.env\n", hash_begin("gitignore/head"))));
    assert!(gitignore.ends_with(&format!("{}foo/\n", hash_end("gitignore/head"))));
    assert_eq!(gitignore.matches("target/").count(), 1);
    let dependabot = fixture.read(".github/dependabot.yml");
    assert!(dependabot.starts_with(&format!(
        "version: 2\nupdates:\n\n{}  - package-ecosystem: github-actions\n",
        hash_begin("dependabot/actions.yml")
    )));
    assert_eq!(dependabot.matches("package-ecosystem").count(), 1);

    // the tables lose their markers and keep the repository's keys and comments
    let cargo = fixture.read("Cargo.toml");
    assert_eq!(cargo.matches("unsafe_code").count(), 1);
    assert_eq!(cargo.matches("missing_docs").count(), 1);
    assert_eq!(keys(&cargo, "workspace.lints.rust").last().unwrap(), "elided_lifetimes_in_paths");
    assert_eq!(value(&cargo, "workspace.lints.clippy.missing_errors_doc"), "\"allow\"");
    assert!(cargo.contains(
        "[workspace.lints.rust]\n# https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html\nmissing_docs = \"warn\"\n"
    ));
    assert!(cargo.contains("\n\n[workspace.lints.clippy]\n"), "{cargo}");
    assert!(cargo.ends_with("\n\n[profile.release]\nlto = true\n"), "{cargo}");
    let deny = fixture.read("deny.toml");
    assert_eq!(value(&deny, "advisories.ignore"), "[]");
    assert!(value(&deny, "licenses.exceptions").contains("unicode_names2"));
    assert_eq!(value(&deny, "licenses.private.ignore"), "true");
    assert_eq!(fixture.read("rust-toolchain.toml"), TOOLCHAIN);
    assert_callers(&fixture);
    assert!(
        fixture
            .read("clippy.toml")
            .contains("# conventions:begin clippy/guest-deny-list\ndisallowed-methods")
    );

    let check = fixture.checkout("check");
    assert!(check.status.success(), "{}", stdout(&check));
    let again = fixture.checkout("sync");
    let out = stdout(&again);
    assert!(!out.contains("wrote") && !out.contains("created"), "{out}");
}

#[test]
fn embedded_tree_matches_the_checkout() {
    let from_checkout = Fixture::new();
    assert!(from_checkout.checkout("sync").status.success());
    let embedded = Fixture::new();
    let sync = embedded.run(&["sync"]);
    assert!(sync.status.success(), "{}", stderr(&sync));
    for file in ["AGENTS.md", "Cargo.toml", "rust-toolchain.toml", "rustfmt.toml", "deny.toml"] {
        assert_eq!(from_checkout.read(file), embedded.read(file), "{file}");
    }
    assert!(embedded.run(&["check"]).status.success());
}

#[test]
fn a_repositorys_own_keys_are_not_held() {
    let fixture = Fixture::new();
    assert!(fixture.checkout("sync").status.success());
    fixture.edit("rust-toolchain.toml", |t| t.replace("wasm32-wasip2", "wasm32-unknown-unknown"));
    fixture.edit("Cargo.toml", |t| {
        t.replace("missing_errors_doc = \"allow\"", "missing_errors_doc = \"warn\"")
    });
    fixture.edit("deny.toml", |t| t.replace("ignore = []", "ignore = [\"RUSTSEC-0000-0000\"]"));
    let check = fixture.checkout("check");
    assert!(check.status.success(), "{}", stdout(&check));
}

type Drift = (&'static str, fn(&Fixture), &'static str);

#[test]
fn check_fails_on_each_class_of_drift() {
    let drifts: [Drift; 10] = [
        (
            "whole file edited",
            |f| f.edit("rustfmt.toml", |t| t.replace("max_width = 100", "max_width = 80")),
            "rustfmt.toml (repository)",
        ),
        (
            "block edited",
            |f| f.edit("AGENTS.md", |t| t.replace("Never `git commit`", "Always `git commit`")),
            "AGENTS.md (repository)",
        ),
        (
            "block removed",
            |f| {
                f.edit("AGENTS.md", |t| {
                    let begin = t.find(&html_begin("agents/git.md")).unwrap();
                    let end = t.find(&html_end("agents/git.md")).unwrap()
                        + html_end("agents/git.md").len();
                    format!("{}{}", &t[..begin], &t[end..])
                });
            },
            "+<!-- BEGIN Managed by augentic/toolkit: conventions/agents/git.md -->",
        ),
        (
            "end marker lost",
            |f| f.edit("AGENTS.md", |t| t.replace(&html_end("agents/git.md"), "")),
            "unresolved: AGENTS.md",
        ),
        (
            "stale pin",
            |f| {
                f.edit(".github/workflows/publish.yaml", |t| {
                    t.replace(&format!("@v{VERSION}"), "@v0.1.0")
                });
            },
            "publish.yaml (repository)",
        ),
        (
            "legacy header back on a caller",
            |f| {
                f.edit(".github/workflows/ci.yaml", |t| {
                    legacy_header("conventions/workflows/ci.yaml") + &t
                });
            },
            "ci.yaml (repository)",
        ),
        (
            "AGENTS.md over budget",
            |f| f.edit("AGENTS.md", |t| t + &"x".repeat(31 * 1024)),
            "rule: AGENTS.md is",
        ),
        (
            "code of conduct misspelled",
            |f| f.write("CODE-OF-CONDUCT.md", "# CoC\n"),
            "rule: CODE-OF-CONDUCT.md",
        ),
        (
            "shared value changed",
            |f| {
                f.edit("Cargo.toml", |t| {
                    t.replace("unsafe_code = \"deny\"", "unsafe_code = \"warn\"")
                });
            },
            "Cargo.toml (repository)",
        ),
        (
            "table left invalid",
            |f| f.edit("Cargo.toml", |t| t.replace("elided_lifetimes_in_paths", "unsafe_code")),
            "unresolved: Cargo.toml",
        ),
    ];
    for (drift, apply, expected) in drifts {
        let fixture = Fixture::new();
        assert!(fixture.checkout("sync").status.success());
        apply(&fixture);
        let check = fixture.checkout("check");
        let out = stdout(&check);
        assert!(!check.status.success(), "{drift}: check passed\n{out}");
        assert!(out.contains(expected), "{drift}: expected `{expected}` in\n{out}");
    }
}

#[test]
fn embedded_program_refuses_a_pin_it_was_not_built_from() {
    let fixture = Fixture::new();
    fixture.edit("mise.toml", |t| t.replace(&format!("?ref=v{VERSION}"), "?ref=v0.0.1"));
    assert!(fixture.run(&["sync"]).status.success());
    let check = fixture.run(&["check"]);
    assert!(!check.status.success());
    assert!(stdout(&check).contains("but mise.toml pins v0.0.1"));
    assert!(
        fixture.read(".github/workflows/ci.yaml").contains(&format!("ci.yaml@v{VERSION}\n")),
        "the pin is the program's, whose tree was written"
    );
}

#[test]
fn no_mise_include_compares_no_pin() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.path("mise.toml")).unwrap();
    let sync = fixture.run(&["sync"]);
    assert!(sync.status.success(), "{}", stderr(&sync));
    assert!(fixture.read(".github/workflows/ci.yaml").contains(&format!("ci.yaml@v{VERSION}\n")));
    let check = fixture.run(&["check"]);
    assert!(check.status.success(), "{}", stdout(&check));
}
