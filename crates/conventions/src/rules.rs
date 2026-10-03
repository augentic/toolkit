use crate::consumer::{Consumer, MISE};
use crate::toolkit::{Toolkit, VERSION};

// Agent loaders cap what they read (Codex at 32 KiB across the chain), and the
// shared blocks must sit inside the cap.
const AGENTS_BUDGET: usize = 30 * 1024;

// The structural rules `check` holds a repository to beyond its managed files.
pub fn failures(toolkit: &Toolkit, consumer: &Consumer) -> Vec<String> {
    let mut out = Vec::new();

    if toolkit.is_embedded()
        && let Some(mise_pin) = &consumer.mise_pin
        && *mise_pin != consumer.pin
    {
        out.push(format!(
            "conventions {VERSION} was built from v{VERSION}, but {MISE} pins {mise_pin}; `make conventions-sync` installs the pinned program"
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

    out
}
