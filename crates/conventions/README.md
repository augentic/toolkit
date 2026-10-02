# conventions

The program behind `make conventions-sync` and `make conventions-check` in
every Augentic Rust repository. It embeds the [`conventions/`](../../conventions)
tree of the toolkit release it is built from and writes or verifies the files
that tree manages in a consuming repository.

See the repository [README](../../README.md#conventions) for the modes, the
markers, and `conventions.toml`.
