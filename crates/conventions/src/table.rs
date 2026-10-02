use anyhow::{Context as _, Result, bail};
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::marker::{self, Style};

// A TOML file with the block `name` holding `body`. Between existing markers
// the body replaces what was there. Otherwise the block is merged in: the
// first table it declares is taken over at its header, the keys the block sets
// leave the file's copy, and the file's remaining keys of that table follow
// the end marker; a block of root keys goes before the first table; a block
// whose tables the file lacks is appended.
pub fn merge(text: &str, name: &str, body: &str) -> Result<String> {
    let lines: Vec<&str> = text.lines().collect();
    let out = if marker::scan(&lines, Style::Hash)?.iter().any(|span| span.name == name) {
        marker::splice(text, Style::Hash, name, body)?
    } else {
        insert(text, name, body)?
    };
    out.parse::<DocumentMut>()
        .with_context(|| format!("the file is not valid TOML with `{name}` in it"))?;
    Ok(out)
}

fn insert(text: &str, name: &str, body: &str) -> Result<String> {
    let block: DocumentMut =
        body.parse().with_context(|| format!("conventions block `{name}` is not valid TOML"))?;
    let mut doc: DocumentMut = text.parse().context("the file is not valid TOML")?;

    let root_keys: Vec<String> = block
        .as_table()
        .iter()
        .filter(|(_, item)| !item.is_table())
        .map(|(key, _)| key.to_owned())
        .collect();
    let mut tables = Vec::new();
    collect_tables(block.as_table(), &mut Vec::new(), &mut tables);
    tables.sort_by_key(|(_, table)| table.position());

    let wrapped = |style: Style| {
        let body = body.strip_suffix('\n').unwrap_or(body);
        format!("{}\n{body}\n{}", style.begin(name), style.end(name))
    };

    if !root_keys.is_empty() {
        if !tables.is_empty() {
            bail!("block `{name}` mixes root keys with tables; a block is one or the other");
        }
        let root = doc.as_table_mut();
        for key in &root_keys {
            root.remove(key);
        }
        let rendered = doc.to_string();
        let block = wrapped(Style::Hash);
        let Some(index) = rendered.lines().position(|line| line.trim_start().starts_with('['))
        else {
            return Ok(append(&rendered, &block));
        };
        let mut lines: Vec<&str> = rendered.lines().collect();
        lines.insert(index, &block);
        lines.insert(index + 1, "");
        return Ok(finish(lines.join("\n")));
    }

    let Some((lead, _)) = tables.first() else {
        bail!("block `{name}` declares neither root keys nor a table");
    };

    // the block replaces the header of the lead table, or of the first table
    // of its that the file has when the lead is absent
    let header_of = |path: &[String]| format!("[{}]", path.join("."));
    let header_line = |text: &str, path: &[String]| {
        let header = header_of(path);
        text.lines().position(|line| line.trim().replace(' ', "") == header)
    };
    let present =
        |doc: &mut DocumentMut, path: &[String]| table_mut(doc.as_table_mut(), path).is_some();
    let anchor = if present(&mut doc, lead) {
        lead.clone()
    } else {
        tables
            .iter()
            .filter_map(|(path, _)| header_line(text, path).map(|line| (line, path)))
            .min_by_key(|(line, _)| *line)
            .map_or_else(|| lead.clone(), |(_, path)| path.clone())
    };

    // the anchor keeps its header for the block to replace; the lead keeps the
    // file's keys beneath the end marker; every other table the block carries
    // whole, so the file's copy must carry nothing more
    for (path, table) in &tables {
        let Some(existing) = table_mut(doc.as_table_mut(), path) else { continue };
        if path != lead
            && let Some((key, _)) = existing.iter().find(|(key, _)| !table.contains_key(key))
        {
            bail!(
                "{} in the file has `{key}`, which block `{name}` does not carry; move it after the block by hand",
                header_of(path)
            );
        }
        if path == lead || path == &anchor {
            let keys: Vec<String> = existing.iter().map(|(key, _)| key.to_owned()).collect();
            for key in keys.iter().filter(|key| path != lead || table.contains_key(key)) {
                existing.remove(key);
            }
        } else {
            remove_table(doc.as_table_mut(), path);
        }
    }

    let rendered = doc.to_string();
    Ok(match header_line(&rendered, &anchor) {
        Some(index) => {
            let mut lines: Vec<&str> = rendered.lines().collect();
            let block = wrapped(Style::Hash);
            lines[index] = &block;
            finish(lines.join("\n"))
        }
        None if present(&mut doc, &anchor) => bail!(
            "{} exists in the file but not as a header line; put the `{name}` markers in place by hand",
            header_of(&anchor)
        ),
        None => append(&rendered, &wrapped(Style::Hash)),
    })
}

// A TOML file holding every key the `body` sets, at the body's value, with no
// markers: a key the file has at another value is reassigned in place, one it
// lacks is added, and the rest of the file is left as it is.
pub fn set(text: &str, body: &str) -> Result<String> {
    let block: DocumentMut = body.parse().context("conventions block is not valid TOML")?;
    let mut doc: DocumentMut = if text.trim().is_empty() {
        DocumentMut::new()
    } else {
        text.parse().context("the file is not valid TOML")?
    };
    set_keys(block.as_table(), doc.as_table_mut());
    Ok(finish(doc.to_string()))
}

fn set_keys(from: &Table, into: &mut Table) {
    for (key, item) in from {
        match item {
            Item::Table(child) => {
                if !into.get(key).is_some_and(Item::is_table) {
                    let mut table = Table::new();
                    table.set_implicit(child.is_implicit());
                    into.insert(key, Item::Table(table));
                }
                if let Some(target) = into.get_mut(key).and_then(Item::as_table_mut) {
                    set_keys(child, target);
                }
            }
            Item::Value(value) => {
                let same = into
                    .get(key)
                    .and_then(Item::as_value)
                    .is_some_and(|have| plain(have) == plain(value));
                if !same {
                    into.insert(key, Item::Value(value.clone()));
                }
            }
            Item::ArrayOfTables(_) | Item::None => {}
        }
    }
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

fn collect_tables<'a>(
    table: &'a Table, path: &mut Vec<String>, out: &mut Vec<(Vec<String>, &'a Table)>,
) {
    for (key, item) in table {
        if let Item::Table(child) = item {
            path.push(key.to_owned());
            if !child.is_implicit() {
                out.push((path.clone(), child));
            }
            collect_tables(child, path, out);
            path.pop();
        }
    }
}

fn table_mut<'a>(root: &'a mut Table, path: &[String]) -> Option<&'a mut Table> {
    let mut table = root;
    for key in path {
        table = table.get_mut(key)?.as_table_mut()?;
    }
    Some(table)
}

fn remove_table(root: &mut Table, path: &[String]) {
    let Some((last, parents)) = path.split_last() else { return };
    if let Some(parent) = table_mut(root, parents) {
        parent.remove(last);
    }
}

fn append(text: &str, block: &str) -> String {
    let mut out = text.trim_end().to_owned();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(block);
    out.push('\n');
    out
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

    const LINTS: &str = "[workspace.lints.rust]\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\n";

    #[test]
    fn takes_over_an_existing_table_and_keeps_local_keys_beneath() {
        let file = "[workspace]\nmembers = [\"a\"]\n\n[workspace.lints.rust]\n# why\nunsafe_code = \"deny\"\nlocal = \"warn\"\n\n[profile.release]\nlto = true\n";
        let out = merge(file, "lints/rust", LINTS).unwrap();
        assert_eq!(
            out,
            "[workspace]\nmembers = [\"a\"]\n\n# conventions:begin lints/rust\n[workspace.lints.rust]\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\n# conventions:end lints/rust\nlocal = \"warn\"\n\n[profile.release]\nlto = true\n"
        );
    }

    #[test]
    fn appends_a_table_the_file_lacks() {
        let out = merge("[workspace]\nmembers = []\n", "lints/rust", LINTS).unwrap();
        assert_eq!(
            out,
            "[workspace]\nmembers = []\n\n# conventions:begin lints/rust\n[workspace.lints.rust]\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\n# conventions:end lints/rust\n"
        );
    }

    #[test]
    fn root_keys_go_before_the_first_table() {
        let file = "# notes\ndoc-valid-idents = [\"X\"]\ndisallowed-types = []\n\n[other]\nk = 1\n";
        let out = merge(file, "clippy/deny", "disallowed-types = [\"a\"]\n").unwrap();
        assert_eq!(
            out,
            "# notes\ndoc-valid-idents = [\"X\"]\n\n# conventions:begin clippy/deny\ndisallowed-types = [\"a\"]\n# conventions:end clippy/deny\n\n[other]\nk = 1\n"
        );
    }

    #[test]
    fn complete_tables_replace_the_files_copies() {
        let file = "[cargo-vet]\nversion = \"0.10\"\n\n[imports.a]\nurl = \"old\"\n\n[imports.b]\nurl = \"b\"\n\n[[exemptions.x]]\nversion = \"1\"\n";
        let out =
            merge(file, "vet/imports", "[imports.a]\nurl = \"new\"\n\n[imports.b]\nurl = \"b\"\n")
                .unwrap();
        assert_eq!(
            out,
            "[cargo-vet]\nversion = \"0.10\"\n\n# conventions:begin vet/imports\n[imports.a]\nurl = \"new\"\n\n[imports.b]\nurl = \"b\"\n# conventions:end vet/imports\n\n[[exemptions.x]]\nversion = \"1\"\n"
        );
    }

    #[test]
    fn anchors_on_the_first_present_table_when_the_lead_is_absent() {
        let file = "[cargo-vet]\nversion = \"0.10\"\n\n[imports.b]\nurl = \"old\"\n\n[[exemptions.x]]\nversion = \"1\"\n";
        let out =
            merge(file, "vet/imports", "[imports.a]\nurl = \"a\"\n\n[imports.b]\nurl = \"b\"\n")
                .unwrap();
        assert_eq!(
            out,
            "[cargo-vet]\nversion = \"0.10\"\n\n# conventions:begin vet/imports\n[imports.a]\nurl = \"a\"\n\n[imports.b]\nurl = \"b\"\n# conventions:end vet/imports\n\n[[exemptions.x]]\nversion = \"1\"\n"
        );
    }

    #[test]
    fn set_adds_and_reassigns_keys_without_markers() {
        let file = "\n# cargo-vet config file\n\n[cargo-vet]\nversion = \"0.10\"\n\n[imports.b]\nurl = \"old\"\n\n[imports.c]\nurl = \"c\"\n\n[[exemptions.x]]\nversion = \"1\"\n";
        let body = "[imports.a]\nurl = \"a\"\n\n[imports.b]\nurl = \"b\"\n";
        let out = set(file, body).unwrap();
        assert_eq!(
            out,
            "\n# cargo-vet config file\n\n[cargo-vet]\nversion = \"0.10\"\n\n[imports.b]\nurl = \"b\"\n\n[imports.c]\nurl = \"c\"\n\n[imports.a]\nurl = \"a\"\n\n[[exemptions.x]]\nversion = \"1\"\n"
        );
        assert_eq!(set(&out, body).unwrap(), out, "holding keys is a fixed point");
        assert_eq!(set("", "[imports.a]\nurl = \"a\"\n").unwrap(), "[imports.a]\nurl = \"a\"\n");
    }

    #[test]
    fn existing_markers_are_refilled() {
        let file = "# conventions:begin lints/rust\n[workspace.lints.rust]\nold = 1\n# conventions:end lints/rust\nlocal = \"warn\"\n";
        let out = merge(file, "lints/rust", LINTS).unwrap();
        assert_eq!(
            out,
            "# conventions:begin lints/rust\n[workspace.lints.rust]\nmissing_docs = \"warn\"\nunsafe_code = \"deny\"\n# conventions:end lints/rust\nlocal = \"warn\"\n"
        );
    }

    #[test]
    fn a_result_that_is_not_toml_is_refused() {
        let file = "# conventions:begin x\n# conventions:end x\n[t]\nk = 1\n";
        merge(file, "x", "[t]\nk = 2\n").unwrap_err();
    }
}
