## 0.5.0

### Removed

- The `conventions/` tree, the `conventions` program and everything that ran
  it: the `conventions` job of `ci.yaml`, the `conventions-sync` and
  `conventions-check` tasks of `mise/rust.toml` (`ci` ends at `deny` and
  `check` at `deps`), and the crate version check of `self-release.yaml`.
  Every file the program wrote is the consumer's own from this tag: delete
  the `Managed by augentic/toolkit` notices and `BEGIN` / `END` markers, and
  bump the `uses:` tags and the mise `?ref=` by hand.
- The Rust workspace that hosted the program, with `self-ci.yaml`, the root
  `mise.toml` and `Makefile`, and the cargo-vet store of its dependencies.
  `supply-chain/audits.toml` stays as the organisation's trusted-publisher
  reference, and `vet-aggregate.yaml` still rebuilds
  `supply-chain/augentic/audits.toml` from it and every consumer's.

## 0.4.0

### Added

- `release.yaml` and `patch.yaml` draft the new `RELEASES.md` section from
  the pull requests merged since the previous tag, through
  `github/copilot-release-notes` under the built-in token, in a `notes` job
  that holds no secret and writes nothing. The draft is one entry per pull
  request under `Added`, `Changed`, `Fixed`, `Removed`, or `Security`, as
  the shared `.github/release-notes-instructions.md` directs (a repository's
  own file at that path takes its place), with an entry the model is unsure
  of under `### Needs Review` and its reason. When nothing is drafted the
  section is the list GitHub generates, as before, with a warning on the run.
- `publish.yaml` refuses to date and tag a `RELEASES.md` that still carries a
  `### Needs Review` heading.
- The `previous-tag` composite action: the highest semver tag strictly below
  a version, shared by the drafting job and the generated-notes fallback.

### Changed

- A caller of `release.yaml` or `patch.yaml` grants `copilot-requests: write`
  under `permissions`; without it the drafting fails and the cut falls back
  to the generated notes. The workflows' `CARGO_REGISTRY_TOKEN` is set on
  the jobs that bump and push rather than on the workflow.
- The `release-notes` composite action takes `previous-tag` and `draft`. With
  a draft, a cut keeps the hand-written entries above the `---` rule, drops
  a `### ` heading with nothing beneath it, and appends the draft; a patch
  section is the draft in place of the generated list under `### Fixed`. The
  `next` skeleton is `## <version>` and `Unreleased` with no headings, since
  the headings come with the draft at the cut.
- The "bump version" pull request says whether the notes on the release
  branch were drafted or generated, and that anything under `### Needs
  Review` is resolved on the release branch before Publish Release.

### Conventions

- Shared TOML tables (`Cargo.toml` lints, `deny.toml`, the vet imports,
  `rust-toolchain.toml`) are owned by key: every key the tree sets holds the
  tree's value wherever the file keeps it, a key the file lacks is added
  beside the shared keys with its comment from the tree, a key the manifest
  lists as `retired` is removed, and every other key of the file is the
  repository's and never moves. The first `sync` drops the
  `conventions:begin` / `conventions:end` pairs 0.3.0 wrote around those
  tables; `sync` and `check` print the repository's own keys of each shared
  table in their place.
- The managed notice is two lines, `Managed by augentic/toolkit:
  conventions/<source>` and ``Do not edit: run `make conventions-sync`.``, on
  every whole file and stub, so the header every consumer carries changes at
  the bump. Block markers carry the same notice behind `BEGIN` and `END`,
  keyed by the source path with its extension
  (`<!-- BEGIN Managed by augentic/toolkit: conventions/agents/git.md -->`);
  a 0.3.0 `conventions:begin` / `conventions:end` pair is respelled in place
  at the first `sync` and is read for this release only.
- `agents/commands.md` states the rule in that one vocabulary: a file that
  opens with `Managed by augentic/toolkit`, everything from a `BEGIN` line to
  its `END` line, and every value the shared TOML tables set, is written by
  `make conventions-sync`.
- `conventions.toml` is gone and nothing reads it; delete the file at the
  bump. Each knob returns to the file that owns it: `targets` is
  `rust-toolchain.toml`'s and the callers' `with:`, `wasm-packages` and
  `outdated-ignore` are `mise.toml [env]` and the callers' `with:`, with no
  checker holding the two equal; `name`, `pinned` and `guest-clippy` have no
  replacement.
- The four caller workflows `ci.yaml`, `audit.yaml`, `patch.yaml` and
  `release.yaml` are the repository's, pin-rewritten like `publish.yaml` and
  never rendered. Stub mode is gone with them; the first `sync` strips the
  0.3.0 header from each caller and from `rust-toolchain.toml`.
- `rust-toolchain.toml` is a shared table owned by key: `channel` and
  `components` hold the tree's values and `targets` is the repository's.
- The guest deny-list (`disallowed-methods`, `disallowed-types`) is no longer
  managed; each guest repository owns the `clippy.toml` it already carries.
- `make conventions-sync`, `make conventions-check` and the `conventions` job
  of `ci.yaml` run over whichever repository invokes them, with no presence
  test; the toolkit root carries the conventions itself, synced from its
  working tree through `CONVENTIONS_TOOLKIT = "."`.
- `lints/rust.toml` and `lints/clippy.toml` carry the comments as omnia
  writes them: no documentation URLs, one line over the restriction picks.
  A comment reaches a consumer only with a key it lacked, so a repository's
  existing comments stand.

## 0.3.0

This repository is now `augentic/toolkit`, moved from `augentic/.github` with
its history and tags; `augentic/.github` keeps the organisation profile and
the default community health files. A consumer re-pins every
`uses: augentic/.github/...@v0.2.0` and the mise `?ref=` to
`augentic/toolkit` at this tag.

### Conventions

- The `conventions/` tree and the `conventions` program
  (`crates/conventions`): the files every consumer carries, in five modes
  (whole file, marker block, TOML table block, rendered stub, pin rewrite),
  written by `make conventions-sync` and held by `make conventions-check`.
  See the README's [Conventions](README.md#conventions).
- Whole files: `rustfmt.toml`, `taplo.toml` (without the commented-out rules
  three repositories carried), `Makefile`, `LICENSE-MIT`, `LICENSE-APACHE`,
  `CODE_OF_CONDUCT.md` (the spelling GitHub recognises; the first sync
  renames `CODE-OF-CONDUCT.md`), `GOVERNANCE.md`, and `renovate.json`, which
  bumps the toolkit pin as one grouped pull request over every `uses:` and
  the mise `?ref=`.
- Blocks: `AGENTS.md` (Git, code style, comments, testing, commands),
  `CONTRIBUTING.md` (DCO, pull request procedure, conduct), the `.gitignore`
  head, and the Dependabot `github-actions` entry, now ignoring `augentic/*`.
  The Git rule reads "unless the maintainer lifts this for the session" and
  covers any sibling checkout.
- TOML tables: `[workspace.lints.rust]` with `missing_docs = "warn"` and
  `unsafe_code = "deny"` org-wide, `[workspace.lints.clippy]` with the lint
  groups at `priority = -1` and the restriction picks, `[licenses] allow`
  as the union of every repository's allowlist (`BSL-1.0` and `CC0-1.0`
  join), `[advisories]`, and the guest deny-list (`disallowed-methods`,
  `disallowed-types`) in each `clippy.toml` that `guest-clippy` names. The
  seven `[imports.*]` of `supply-chain/config.toml` (`augentic` among them)
  are held as keys without markers, since `cargo vet` rewrites that file.
- Stubs: the `ci.yaml`, `audit.yaml`, `patch.yaml`, and `release.yaml`
  callers (one concurrency shape; `release.yaml` takes an `increment` choice)
  and `rust-toolchain.toml` (`targets` from `conventions.toml`; the host
  target four repositories listed is dropped, since every command runs on the
  host anyway).

### Added

- A Rust workspace at the root for `crates/conventions`, with this
  repository's own `self-ci.yaml` calling the reusable `ci.yaml` at the
  commit under test.
- `ci.yaml` gains a `conventions` job: it builds the program from the toolkit
  revision the caller's `uses:` names and runs `conventions check`, so a
  drifted file or a pin out of step with the `uses:` fails CI. A repository
  without `conventions.toml` passes with a notice.
- `mise/rust.toml` gains `conventions-sync` and `conventions-check`, which
  build the program once per pin from the `?ref=` tag into `target/toolkit/`
  (`CONVENTIONS_TOOLKIT=<checkout>` runs a checkout instead), and `ci` and
  `check` end with `conventions-check`.
- `self-release.yaml` refuses a release whose `crates/conventions` version is
  not the one on line 1 of `RELEASES.md`, since the program reports that
  version as the pin it was built from.
- The organisation's aggregated cargo-vet audits at
  `supply-chain/augentic/audits.toml`, rebuilt weekly by `vet-aggregate.yaml`
  from the `audits.toml` of every consumer and of this repository
  (`supply-chain/augentic/sources.list`) and imported by every consumer as
  `[imports.augentic]`. cargo-vet does not import `[[trusted.*]]` entries, so
  each repository keeps its own; this repository's `supply-chain/audits.toml`
  holds the union of the organisation's trusted publishers as the reference
  to copy from, renewed to October 2027.
- `LICENSE-MIT` and `LICENSE-APACHE`: the repository is licensed as the
  consumers are, MIT OR Apache-2.0.

### Changed

- `mise/rust.toml`: `test-docs` mirrors the CI `test-doc` job's library
  detection and runs nothing in a workspace without a library target, where
  `cargo test --doc` fails instead.
- README rewritten for the new repository: consumer configuration now covers
  `conventions.toml` and the sync and check tasks, and the mirror table gains
  the `conventions` job.

## 0.2.0

### Added

- `ci.yaml`: new `wasm-packages` input (space-separated workspace members,
  default empty = whole workspace) narrowing which packages the per-target
  clippy passes lint. Workspaces that mix host-only crates with guest
  components can now pass `targets: wasm32-wasip2` at all.
- `mise/rust.toml`: `lint-host` and `lint-wasm` (the two halves of `lint`),
  `fmt-check`, `vet-regen` and a `cov` coverage helper
  (`cargo llvm-cov nextest --workspace --all-features --summary-only`).
- `mise/rust.toml`: consumer knobs read from `[env]`: `WASM32_PACKAGES`
  (mirrors `wasm-packages`) and `OUTDATED_IGNORE` (comma-separated, mirrors
  `audit.yaml`'s `outdated` input).
- README: consumer configuration, the wasm32 lint convention and a table
  mapping each `ci.yaml` job to its `mise/rust.toml` task.

### Changed

- `ci.yaml` and `mise/rust.toml` (`lint-wasm`): the per-target clippy pass
  builds `--lib --bins --examples` instead of `--all-targets`, and the
  per-target `cargo hack clippy --each-feature` pass uses cargo's default
  targets (lib + bins). Integration tests and benches, whose dev-dependencies
  are host-only in most workspaces, are no longer compiled for wasm32.
  Convention: any bin or example in the wasm32 scope that is host-only must be
  cfg-gated to an empty `main` on wasm32.
- `mise/rust.toml`: `vet` is now check-only (`cargo vet --locked`), matching
  the CI `vet` job; the three `regenerate` steps moved to `vet-regen`.
- `mise/rust.toml`: `ci` runs exactly the CI jobs
  (`fmt-check`, `lint`, `test`, `test-docs`, `docs`, `vet`, `deny`).
  `outdated` and `deps` remain in `check`; `fmt` no longer rewrites sources
  as part of `ci`.
- `mise/rust.toml`: `outdated` no longer hard-codes omnia's ignore list; set
  `OUTDATED_IGNORE` instead.
- `mise/rust.toml`: `lint` passes `--locked`; `RUSTFLAGS=-Dwarnings` is set
  on `lint`, `test`, `test-docs` and `docs`, and `docs` also sets
  `RUSTDOCFLAGS=-Dwarnings`, so `mise run ci` matches the workflow's global
  environment.

## 0.1.2

### Changed

- `ci.yaml`: the `test` job runs `cargo nextest run --workspace --all-features`
  once instead of `cargo hack nextest run --each-feature`. The per-feature
  matrix rebuilt heavy dependencies for every crate/feature pair and ran the
  slow integration tests serially, roughly doubling the job while executing
  exactly the same set of tests; feature-gated tests in the consuming
  repositories are all positively gated, so `--all-features` is a superset.
  Per-feature compile coverage remains in the `clippy` job.
- `mise/rust.toml`: the `test` task makes the same change so it keeps
  mirroring CI, and no longer depends on `cargo-hack`.

## 0.1.1

### Changed

- `ci.yaml`: the `test` job no longer passes `--tests` to `cargo hack nextest run`,
  which was building every test target per feature combination causing excessive
  run time for little benefit.

## 0.1.0

### Added

- Reusable Rust workflows: `ci.yaml`, `audit.yaml`, `release.yaml`,
  `patch.yaml`, `publish.yaml`, `crates.yaml`, `wasm.yaml`.
- Composite actions: `git-identity`, `cargo-version`, `release-notes`.
- Shared mise tasks for Rust workspaces (`mise/rust.toml`).
- `lint.yaml` running actionlint over this repository's workflows.
- Tagged releases of this repository via the `Release` workflow.
- `release.yaml`, `publish.yaml` and `patch.yaml` resolve their composite
  actions at the same commit as the workflow, so pinning a tag pins the
  actions too.
