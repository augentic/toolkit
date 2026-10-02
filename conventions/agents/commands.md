## Commands

All from the repository root through `make` ([`Makefile`](Makefile) → mise). The tasks are the shared `mise/rust.toml` of [`augentic/toolkit`](https://github.com/augentic/toolkit), pinned in [`mise.toml`](mise.toml) to the tag every `uses:` under `.github/workflows/` names; a bump is one pull request over both, and `make conventions-check` holds them together.

```bash
make ci # exactly the CI jobs: fmt-check + lint + test + test-docs + docs + vet + deny + conventions-check — run before handing over
make check # local advisories: audit + fmt (rewrites) + lint + outdated + deps
make test # cargo nextest run --locked --workspace --all-features, under -Dwarnings
make lint # lint-host (cargo clippy --workspace --all-targets --all-features, then cargo hack --each-feature), then lint-wasm (the same over every lib, bin and example for wasm32-wasip2 — never tests)
make fmt # cargo +nightly fmt --all
make vet-regen # regenerate cargo-vet imports/exemptions/unpublished, then vet
make conventions-sync # write the shared conventions at the pinned toolkit tag
make cov # cargo llvm-cov nextest --workspace --all-features --summary-only
make sweep # drop target/ artifacts untouched for a week
```

A file with a `Managed by augentic/toolkit` header, and everything between a `conventions:begin` and `conventions:end` marker pair, is written by `make conventions-sync`: never edit it here. Change it in [`augentic/toolkit`](https://github.com/augentic/toolkit) instead. If `make ci` cannot run, say exactly why and which checks ran instead.
