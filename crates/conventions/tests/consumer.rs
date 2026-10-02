//! Drives the `conventions` binary over a fixture consumer with the
//! repository's own `conventions/` tree.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    // A consumer that already carries unmarked copies of what the blocks
    // bring, local keys in the shared tables, and stale pins.
    fn new() -> Self {
        let fixture = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fixture.write(
            "conventions.toml",
            "name = \"fixture\"\ntargets = [\"wasm32-wasip2\"]\nwasm-packages = [\"guest\"]\nguest-clippy = [\"clippy.toml\"]\npinned = [\"docs/template.yaml\"]\n",
        );
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
        fixture.write("clippy.toml", "doc-valid-idents = [\"GitHub\"]\n");
        fixture.write("AGENTS.md", "# Fixture\n\nWhat this repository is.\n");
        fixture.write(".gitignore", "target/\nfoo/\n");
        fixture.write(
            ".github/workflows/publish.yaml",
            "name: Publish\njobs:\n  publish:\n    uses: augentic/toolkit/.github/workflows/publish.yaml@v0.1.0\n",
        );
        fixture.write(
            "docs/template.yaml",
            "uses: augentic/toolkit/.github/workflows/ci.yaml@v0.0.1\n",
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
        let toolkit = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        self.run(&[command, "--toolkit", toolkit.to_str().unwrap()])
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn after(text: &str, marker: &str) -> String {
    let at = text.find(marker).unwrap_or_else(|| panic!("{marker} not in:\n{text}"));
    text[at + marker.len()..].to_owned()
}

#[test]
fn sync_then_check() {
    let fixture = Fixture::new();
    let sync = fixture.checkout("sync");
    assert!(sync.status.success(), "{}", stderr(&sync));

    // whole files carry the header
    assert!(
        fixture
            .read("rustfmt.toml")
            .starts_with("# Managed by augentic/toolkit from conventions/rustfmt.toml.")
    );
    assert!(fixture.read("GOVERNANCE.md").starts_with("<!-- Managed by augentic/toolkit"));
    assert!(fixture.read("CODE_OF_CONDUCT.md").contains("## Community Code of Conduct"));
    assert!(!fixture.read("LICENSE-MIT").contains("Managed by"));

    // blocks sit after the repository's text, a missing file is created
    let agents = fixture.read("AGENTS.md");
    assert!(agents.starts_with(
        "# Fixture\n\nWhat this repository is.\n\n<!-- conventions:begin agents/git -->\n## Git\n"
    ));
    assert!(agents.contains("<!-- conventions:end agents/commands -->\n"));
    assert!(
        fixture
            .read("CONTRIBUTING.md")
            .starts_with("# Contributing\n\n<!-- conventions:begin contributing/dco -->")
    );
    assert_eq!(after(&fixture.read(".gitignore"), "# conventions:end gitignore/head\n"), "");
    assert!(
        fixture
            .read(".gitignore")
            .starts_with("target/\nfoo/\n\n# conventions:begin gitignore/head\n")
    );
    assert!(fixture.read(".github/dependabot.yml").starts_with("version: 2\nupdates:\n\n# conventions:begin dependabot/actions\n  - package-ecosystem: github-actions\n"));

    // tables are taken over, local keys follow the end marker, the file parses
    let cargo = fixture.read("Cargo.toml");
    cargo.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(cargo.matches("unsafe_code").count(), 1);
    assert!(
        after(&cargo, "# conventions:end lints/rust\n")
            .starts_with("elided_lifetimes_in_paths = \"warn\"\n")
    );
    assert!(
        after(&cargo, "# conventions:end lints/clippy\n")
            .starts_with("missing_errors_doc = \"allow\"\n")
    );
    assert!(cargo.contains("\n[profile.release]\nlto = true\n"));
    let deny = fixture.read("deny.toml");
    deny.parse::<toml_edit::DocumentMut>().unwrap();
    assert!(after(&deny, "# conventions:end deny/licenses\n").starts_with("exceptions = ["));
    assert!(after(&deny, "# conventions:end deny/advisories\n").starts_with("ignore = []\n"));
    assert!(deny.contains("[licenses.private]\nignore = true\n"));
    let vet = fixture.read("supply-chain/config.toml");
    vet.parse::<toml_edit::DocumentMut>().unwrap();
    assert_eq!(vet.matches("[imports.mozilla]").count(), 1);
    assert!(!vet.contains("https://old"));
    assert!(vet.contains("[imports.augentic]"));
    assert!(vet.contains("[[exemptions.x]]"));
    let clippy = fixture.read("clippy.toml");
    assert!(clippy.starts_with("doc-valid-idents = [\"GitHub\"]\n\n# conventions:begin clippy/guest-deny-list\ndisallowed-methods = ["));

    // stubs render the configuration, empty values and their parents go
    let ci = fixture.read(".github/workflows/ci.yaml");
    assert!(ci.starts_with(
        "# Managed by augentic/toolkit from conventions/workflows/ci.yaml and conventions.toml."
    ));
    assert!(ci.contains(&format!("uses: augentic/toolkit/.github/workflows/ci.yaml@v{VERSION}\n")));
    assert!(ci.contains("    with:\n      targets: wasm32-wasip2\n      wasm-packages: guest\n"));
    let audit = fixture.read(".github/workflows/audit.yaml");
    assert!(!audit.contains("with:"), "{audit}");
    assert!(
        fixture
            .read(".github/workflows/release.yaml")
            .contains("increment: ${{ inputs.increment }}")
    );
    assert!(fixture.read("rust-toolchain.toml").ends_with("targets = [\"wasm32-wasip2\"]\n"));

    // pins are rewritten in the other workflows and the named files
    assert!(
        fixture
            .read(".github/workflows/publish.yaml")
            .contains(&format!("publish.yaml@v{VERSION}\n"))
    );
    assert!(fixture.read("docs/template.yaml").contains(&format!("ci.yaml@v{VERSION}\n")));

    let check = fixture.checkout("check");
    assert!(check.status.success(), "{}", stdout(&check));
    assert!(stdout(&check).contains("managed files match"));

    let again = fixture.checkout("sync");
    assert!(again.status.success());
    assert_eq!(stdout(&again), "", "a second sync writes nothing");
}

#[test]
fn embedded_tree_matches_the_checkout() {
    let from_checkout = Fixture::new();
    assert!(from_checkout.checkout("sync").status.success());
    let embedded = Fixture::new();
    let sync = embedded.run(&["sync"]);
    assert!(sync.status.success(), "{}", stderr(&sync));
    for file in
        ["AGENTS.md", "Cargo.toml", ".github/workflows/ci.yaml", "rustfmt.toml", "deny.toml"]
    {
        assert_eq!(from_checkout.read(file), embedded.read(file), "{file}");
    }
    assert!(embedded.run(&["check"]).status.success());
}

type Drift = (&'static str, fn(&Fixture), &'static str);

#[test]
fn check_fails_on_each_class_of_drift() {
    let drifts: [Drift; 9] = [
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
                    let begin = t.find("<!-- conventions:begin agents/git -->").unwrap();
                    let end = t.find("<!-- conventions:end agents/git -->").unwrap()
                        + "<!-- conventions:end agents/git -->\n".len();
                    format!("{}{}", &t[..begin], &t[end..])
                });
            },
            "+<!-- conventions:begin agents/git -->",
        ),
        (
            "end marker lost",
            |f| f.edit("AGENTS.md", |t| t.replace("<!-- conventions:end agents/git -->\n", "")),
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
            "env mirror broken",
            |f| {
                f.edit("mise.toml", |t| {
                    t.replace("WASM32_PACKAGES = \"guest\"", "WASM32_PACKAGES = \"other\"")
                });
            },
            "rule: mise.toml [env] WASM32_PACKAGES",
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
        fixture.read(".github/workflows/ci.yaml").contains("ci.yaml@v0.0.1\n"),
        "the pin is mise.toml's, not the program's"
    );
}

#[test]
fn missing_configuration_is_an_error() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.path("conventions.toml")).unwrap();
    let check = fixture.checkout("check");
    assert!(!check.status.success());
    assert!(stderr(&check).contains("conventions.toml is required"));
}
