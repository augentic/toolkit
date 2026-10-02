use toml_edit::DocumentMut;

use crate::config::{CONFIG, Consumer, MISE};
use crate::toolkit::{Toolkit, VERSION};

// Agent loaders cap what they read (Codex at 32 KiB across the chain), and the
// shared blocks must sit inside the cap.
const AGENTS_BUDGET: usize = 30 * 1024;

// The structural rules `check` holds a repository to beyond its managed files.
pub fn failures(toolkit: &Toolkit, consumer: &Consumer) -> Vec<String> {
    let mut out = Vec::new();

    if toolkit.is_embedded() && consumer.pin != Toolkit::pin() {
        out.push(format!(
            "conventions {VERSION} was built from v{VERSION}, but {MISE} pins {}; `make conventions-sync` installs the pinned program",
            consumer.pin
        ));
    }

    if let Ok(agents) = std::fs::read(consumer.root.join("AGENTS.md"))
        && agents.len() > AGENTS_BUDGET
    {
        out.push(format!(
            "AGENTS.md is {} bytes; the budget is {AGENTS_BUDGET} (agent loaders stop reading at 32 KiB)",
            agents.len()
        ));
    }

    if let Ok(entries) = std::fs::read_dir(&consumer.root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name != "CODE_OF_CONDUCT.md"
                && name.to_ascii_uppercase().replace('-', "_") == "CODE_OF_CONDUCT.MD"
            {
                out.push(format!("{name} is spelled so that GitHub does not see it; `git mv {name} CODE_OF_CONDUCT.md`"));
            }
        }
    }

    if let Some(mise) = &consumer.mise {
        match mise.parse::<DocumentMut>() {
            Ok(doc) => {
                let env = |key: &str| {
                    doc.get("env")
                        .and_then(|env| env.get(key))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_owned()
                };
                for (key, config_key, value) in [
                    ("WASM32_PACKAGES", "wasm-packages", consumer.config.wasm_packages.join(" ")),
                    (
                        "OUTDATED_IGNORE",
                        "outdated-ignore",
                        consumer.config.outdated_ignore.join(","),
                    ),
                ] {
                    let actual = env(key);
                    if actual != value {
                        out.push(format!(
                            "{MISE} [env] {key} is `{actual}`; {CONFIG} `{config_key}` renders `{value}`, and the two mirror each other"
                        ));
                    }
                }
            }
            Err(error) => out.push(format!("{MISE} is not valid TOML: {error}")),
        }
    }

    out
}
