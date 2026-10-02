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
