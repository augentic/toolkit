const REPOSITORY: &str = "augentic/toolkit/";

// The text with every `augentic/toolkit/<path>@<ref>` reference at `pin`.
pub fn rewrite(text: &str, pin: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(REPOSITORY) {
        let after = at + REPOSITORY.len();
        out.push_str(&rest[..after]);
        rest = &rest[after..];
        let path_end = rest
            .find(|c: char| c == '@' || c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        if !rest[path_end..].starts_with('@') {
            continue;
        }
        let ref_start = path_end + 1;
        let ref_end = rest[ref_start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/')))
            .map_or(rest.len(), |len| ref_start + len);
        out.push_str(&rest[..ref_start]);
        out.push_str(pin);
        rest = &rest[ref_end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::rewrite;

    #[test]
    fn rewrites_every_reference_and_nothing_else() {
        let text = "uses: augentic/toolkit/.github/workflows/ci.yaml@v0.2.0\n\
                    uses: augentic/toolkit/.github/actions/git-identity@main # note\n\
                    includes = [\"git::https://github.com/augentic/toolkit.git//mise/rust.toml?ref=v0.2.0\"]\n\
                    see augentic/toolkit/README.md\n";
        assert_eq!(
            rewrite(text, "v0.3.0"),
            "uses: augentic/toolkit/.github/workflows/ci.yaml@v0.3.0\n\
             uses: augentic/toolkit/.github/actions/git-identity@v0.3.0 # note\n\
             includes = [\"git::https://github.com/augentic/toolkit.git//mise/rust.toml?ref=v0.2.0\"]\n\
             see augentic/toolkit/README.md\n"
        );
    }
}
