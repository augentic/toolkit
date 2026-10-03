use std::collections::HashMap;

use anyhow::{Context as _, Result, bail};
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::marker::Style;

// A TOML file holding every key the `body` sets, at the body's value, with
// nothing else changed: a key the file lacks arrives with the body's comment,
// beside the body's keys the file has; one at another value is reassigned
// under the file's own key and comment; the file's keys keep their places.
// The marker lines a previous release wrote around the block are dropped.
pub fn set(text: &str, body: &str) -> Result<String> {
    let block = block(body)?;
    let mut doc = parse(text)?;
    set_keys(block.as_table(), doc.as_table_mut(), &mut Vec::new())?;
    Ok(finish(doc.to_string()))
}

// The file without the dotted `paths` a body once set; one the file lacks is nothing.
pub fn retire(text: &str, paths: &[String]) -> Result<String> {
    let mut doc = parse(text)?;
    for path in paths {
        let keys: Vec<&str> = path.split('.').collect();
        if let Some((last, parents)) = keys.split_last()
            && let Some(parent) = table_mut(doc.as_table_mut(), parents)
        {
            parent.remove(last);
        }
    }
    Ok(finish(doc.to_string()))
}

// The repository's own values in one table the body sets values in: the
// table's header, none for the root, and the keys.
pub struct Local {
    pub header: Option<String>,
    pub keys: Vec<String>,
}

pub fn local(text: &str, body: &str) -> Result<Vec<Local>> {
    let block = block(body)?;
    let doc = parse(text)?;
    let mut out = Vec::new();
    collect_local(block.as_table(), doc.as_table(), &mut Vec::new(), &mut out);
    Ok(out)
}

fn block(body: &str) -> Result<DocumentMut> {
    body.parse().context("conventions block is not valid TOML")
}

fn parse(text: &str) -> Result<DocumentMut> {
    let text = unmarked(text);
    if text.trim().is_empty() {
        return Ok(DocumentMut::new());
    }
    text.parse().context("the file is not valid TOML")
}

// The text without its marker lines, one blank line kept between the tables a
// marker separated and never two.
fn unmarked(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if Style::Hash.parse(line).is_none() {
            out.push(line);
            continue;
        }
        let after_blank = out.last().is_none_or(|last| last.trim().is_empty());
        match lines.peek() {
            Some(next) if after_blank && next.trim().is_empty() => {
                lines.next();
            }
            Some(next) if !after_blank && next.trim_start().starts_with('[') => out.push(""),
            _ => {}
        }
    }
    out.join("\n") + "\n"
}

fn set_keys(from: &Table, into: &mut Table, path: &mut Vec<String>) -> Result<()> {
    let mut added = Vec::new();
    for (name, item) in from {
        match item {
            Item::Table(child) => {
                if !into.contains_key(name) {
                    let mut table = Table::new();
                    table.set_implicit(true);
                    into.insert(name, Item::Table(table));
                }
                let Some(target) = into.get_mut(name).and_then(Item::as_table_mut) else {
                    bail!("{} is not a table in the file", dotted(path, name));
                };
                if !child.is_implicit() {
                    target.set_implicit(false);
                }
                path.push(name.to_owned());
                set_keys(child, target, path)?;
                path.pop();
            }
            Item::Value(value) => match into.get_mut(name) {
                Some(Item::Value(have)) => {
                    if plain(have) != plain(value) {
                        *have = value.clone();
                    }
                }
                Some(_) => bail!("{} is not a value in the file", dotted(path, name)),
                None => {
                    if let Some(key) = from.key(name) {
                        into.insert_formatted(key, item.clone());
                        added.push(name.to_owned());
                    }
                }
            },
            Item::ArrayOfTables(_) | Item::None => {}
        }
    }
    if !added.is_empty() {
        let shared: Vec<&str> =
            from.iter().filter(|(_, item)| item.is_value()).map(|(name, _)| name).collect();
        order(into, &shared, &added);
    }
    Ok(())
}

// Keeps the body's keys together: a key the file lacked goes after the body's
// keys the file has before it, else before the ones it has after it, else at
// the end of the table; the file's own keys keep their order.
fn order(table: &mut Table, shared: &[&str], added: &[String]) {
    let kept: Vec<String> = table
        .iter()
        .map(|(name, _)| name.to_owned())
        .filter(|name| !added.contains(name))
        .collect();
    let at = |name: &str| kept.iter().position(|kept| kept == name);
    let mut rank: HashMap<String, (usize, usize)> =
        kept.iter().enumerate().map(|(index, name)| (name.clone(), (index + 1, 0))).collect();
    for (index, name) in shared.iter().enumerate() {
        if !added.iter().any(|added| added == name) {
            continue;
        }
        let before = shared[..index].iter().rev().find_map(|earlier| at(earlier));
        let after = shared[index + 1..].iter().find_map(|later| at(later));
        let line = match (before, after) {
            (Some(at), _) => at + 1,
            (None, Some(at)) => at,
            (None, None) => kept.len() + 1,
        };
        rank.insert((*name).to_owned(), (line, index + 1));
    }
    table.sort_values_by(|a, _, b, _| rank.get(a.get()).cmp(&rank.get(b.get())));
}

fn collect_local(from: &Table, into: &Table, path: &mut Vec<String>, out: &mut Vec<Local>) {
    if from.iter().any(|(_, item)| item.is_value()) {
        let keys: Vec<String> = into
            .iter()
            .filter(|(name, item)| item.is_value() && !from.contains_key(name))
            .map(|(name, _)| name.to_owned())
            .collect();
        if !keys.is_empty() {
            let header = (!path.is_empty()).then(|| format!("[{}]", path.join(".")));
            out.push(Local { header, keys });
        }
    }
    for (name, item) in from {
        if let (Item::Table(child), Some(Item::Table(have))) = (item, into.get(name)) {
            path.push(name.to_owned());
            collect_local(child, have, path, out);
            path.pop();
        }
    }
}

fn dotted(path: &[String], name: &str) -> String {
    path.iter().map(String::as_str).chain([name]).collect::<Vec<_>>().join(".")
}

fn table_mut<'a>(root: &'a mut Table, path: &[&str]) -> Option<&'a mut Table> {
    let mut table = root;
    for key in path {
        table = table.get_mut(key)?.as_table_mut()?;
    }
    Some(table)
}

// A value's text without its decor, so two spellings of one value compare equal.
fn plain(value: &Value) -> String {
    let mut value = value.clone();
    strip(&mut value);
    value.to_string()
}

fn strip(value: &mut Value) {
    value.decor_mut().clear();
    match value {
        Value::Array(array) => {
            array.set_trailing("");
            array.set_trailing_comma(false);
            array.iter_mut().for_each(strip);
        }
        Value::InlineTable(table) => {
            table.set_trailing("");
            table.set_trailing_comma(false);
            for (mut key, value) in table.iter_mut() {
                key.leaf_decor_mut().clear();
                strip(value);
            }
        }
        _ => {}
    }
}

fn finish(mut text: String) -> String {
    while text.ends_with('\n') {
        text.pop();
    }
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINTS: &str = "[workspace.lints.rust]\n# listing\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\nunused_extern_crates = \"warn\"\n";

    #[test]
    fn holds_the_bodys_keys_together_and_leaves_the_files_own() {
        let file = "[workspace]\nmembers = [\"a\"]\n\n[workspace.lints.rust]\n# why\nunsafe_code = \"forbid\"\nlocal = \"warn\"\n\n[profile.release]\nlto = true\n";
        let out = set(file, LINTS).unwrap();
        assert_eq!(
            out,
            "[workspace]\nmembers = [\"a\"]\n\n[workspace.lints.rust]\n# listing\nmissing_docs = \"warn\"\n# why\nunsafe_code = \"deny\"\nunused_extern_crates = \"warn\"\nlocal = \"warn\"\n\n[profile.release]\nlto = true\n"
        );
        assert_eq!(set(&out, LINTS).unwrap(), out, "holding is a fixed point");
    }

    #[test]
    fn adds_a_table_the_file_lacks() {
        let out = set("[workspace]\nmembers = []\n", LINTS).unwrap();
        assert_eq!(
            out,
            "[workspace]\nmembers = []\n\n[workspace.lints.rust]\n# listing\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\nunused_extern_crates = \"warn\"\n"
        );
        assert_eq!(set("", LINTS).unwrap(), LINTS);
    }

    #[test]
    fn drops_the_marker_lines_a_previous_release_wrote() {
        let file = "[workspace]\nmembers = []\n\n# conventions:begin lints/rust\n[workspace.lints.rust]\n# listing\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\nunused_extern_crates = \"warn\"\n# conventions:end lints/rust\nlocal = \"warn\"\n# conventions:begin other\n[other]\nk = 1\n# conventions:end other\n\n[last]\nk = 2\n";
        let out = set(file, LINTS).unwrap();
        assert_eq!(
            out,
            "[workspace]\nmembers = []\n\n[workspace.lints.rust]\n# listing\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\nunused_extern_crates = \"warn\"\nlocal = \"warn\"\n\n[other]\nk = 1\n\n[last]\nk = 2\n"
        );
    }

    #[test]
    fn root_keys_sit_beside_the_files_own() {
        let file = "# notes\ndoc-valid-idents = [\"X\"]\n\n# conventions:begin clippy/guest-deny-list\ndisallowed-types = []\n# conventions:end clippy/guest-deny-list\n";
        let out =
            set(file, "disallowed-methods = [\"m\"]\n\ndisallowed-types = [\"t\"]\n").unwrap();
        assert_eq!(
            out,
            "# notes\ndoc-valid-idents = [\"X\"]\ndisallowed-methods = [\"m\"]\n\ndisallowed-types = [\"t\"]\n"
        );
    }

    #[test]
    fn reassigns_tables_of_keys_in_place_and_adds_the_missing_ones() {
        let file = "\n# cargo-vet config file\n\n[cargo-vet]\nversion = \"0.10\"\n\n[imports.b]\nurl = \"old\"\n\n[imports.c]\nurl = \"c\"\n\n[[exemptions.x]]\nversion = \"1\"\n";
        let body = "[imports.a]\nurl = \"a\"\n\n[imports.b]\nurl = \"b\"\n";
        let out = set(file, body).unwrap();
        assert_eq!(
            out,
            "\n# cargo-vet config file\n\n[cargo-vet]\nversion = \"0.10\"\n\n[imports.b]\nurl = \"b\"\n\n[imports.c]\nurl = \"c\"\n\n[imports.a]\nurl = \"a\"\n\n[[exemptions.x]]\nversion = \"1\"\n"
        );
        assert_eq!(set(&out, body).unwrap(), out, "holding is a fixed point");
    }

    #[test]
    fn retire_removes_what_a_body_no_longer_sets() {
        let file = "[a]\nk = 1\nold = 2\n\n[a.b]\ngone = true\n";
        let paths = ["a.old", "a.b.gone", "a.none", "x.y"].map(str::to_owned);
        assert_eq!(retire(file, &paths).unwrap(), "[a]\nk = 1\n\n[a.b]\n");
    }

    #[test]
    fn local_names_the_files_own_values_in_each_table_the_body_sets() {
        let file = "mine = 1\nshared = 1\n\n[a]\nk = 1\nown = 2\n\n[a.sub]\nx = 1\n\n[b]\nk = 1\n";
        let body = "shared = 1\n\n[a]\nk = 1\n\n[c]\nk = 1\n";
        let shown: Vec<(Option<String>, Vec<String>)> =
            local(file, body).unwrap().into_iter().map(|own| (own.header, own.keys)).collect();
        assert_eq!(
            shown,
            vec![(None, vec!["mine".to_owned()]), (Some("[a]".to_owned()), vec!["own".to_owned()])]
        );
    }

    #[test]
    fn a_file_the_body_cannot_hold_is_refused() {
        set("[t\n", "[t]\nk = 1\n").unwrap_err();
        set("[t]\nk = { a = 1 }\n", "[t.k]\nb = 2\n").unwrap_err();
        set("[t.k]\nb = 2\n", "[t]\nk = 1\n").unwrap_err();
    }
}
