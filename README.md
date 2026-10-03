# Toolkit

The shared engineering conventions of Augentic's Rust repositories: reusable
GitHub workflows, composite actions, mise tasks, and the `conventions/` tree a
repository syncs into its own files. Consumers pin one release tag
(`@vX.Y.Z`) rather than `@main`; see [Versioning](#versioning).

Formerly `augentic/.github`, which now holds the organisation profile and
default community health files alone. Tags up to `v0.2.0` resolve from both
repositories.

## Versioning

This repository is released as `vX.Y.Z` tags with a matching GitHub release.
While on `0.x`, a **minor** bump signals a breaking change (renamed inputs,
changed behaviour, removed workflows, a convention a consumer must act on) and
a **patch** bump is a fix. Release notes live in [RELEASES.md](RELEASES.md).

A consumer carries the tag in two places, and `conventions check` holds every
`uses:` to the mise `?ref=`:

```yaml
jobs:
  ci:
    uses: augentic/toolkit/.github/workflows/ci.yaml@v0.3.0
```

```toml
# mise.toml
[task_config]
includes = ["git::https://github.com/augentic/toolkit.git//mise/rust.toml?ref=v0.3.0"]
```

Because `release.yaml`, `publish.yaml` and `patch.yaml` resolve their
composite actions at their own commit (see
[Composite actions in reusable workflows](#composite-actions-in-reusable-workflows)),
and the `conventions` program is built from the pinned tag, pinning the tag
pins everything it runs and everything it writes.

[Renovate](https://docs.renovatebot.com) bumps the pin: the synced
`renovate.json` groups the `uses:` tags and the `?ref=` into one pull request.
Dependabot's `github-actions` entry stays for the third-party actions a
repository's own workflows use and ignores `augentic/*`.

`@main` still works for trying unreleased changes but is not the supported
reference: it can change under a consumer at any time.

## Consumer configuration

A Rust repository adopts the shared tasks, CI and conventions with its
`mise.toml`, the synced `Makefile`, and its own caller workflows:

```toml
# mise.toml — the one pin, plus the knobs the shared tasks read and any
# repository-specific tasks (a local task shadows a shared one of the same name)
[task_config]
includes = ["git::https://github.com/augentic/toolkit.git//mise/rust.toml?ref=v0.4.0"]

[env]
# Space-separated workspace members `lint-wasm` clippies for wasm32-wasip2.
# Unset = whole workspace. The value `ci.yaml` passes as `wasm-packages`.
WASM32_PACKAGES = "guest-crate examples"
# Comma-separated crates `outdated` ignores. The value `audit.yaml` passes as
# `outdated`.
OUTDATED_IGNORE = "cap-std,cap-primitives"
```

```makefile
# Makefile — synced from the tree: forwards `make <task>` to `mise run <task>`
# and never installs mise itself
```

```yaml
# .github/workflows/ci.yaml — the repository's own caller; the toolkit
# rewrites the pin and nothing else
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  ci:
    uses: augentic/toolkit/.github/workflows/ci.yaml@v0.4.0
    secrets: inherit
    with:
      targets: wasm32-wasip2
      wasm-packages: guest-crate examples
```

`audit.yaml`, `patch.yaml`, `release.yaml` and `publish.yaml` take the same
shape, each calling its reusable workflow with the inputs the repository
chooses. Then `make conventions-sync` writes every managed file (see
[Conventions](#conventions)). `make ci` runs the same checks as the workflow,
`conventions-check` among them; `make check` adds the local-only advisories
(`audit`, `outdated`, `deps`) and rewrites formatting in place.

### The wasm32 lint pass

The host clippy passes never compile code behind
`cfg(target_arch = "wasm32")`, so when `targets` includes `wasm32-wasip2`
(workflow) or `lint-wasm` runs (mise) clippy runs again for that target.
The pass has one shape everywhere:

- **Scope** is the whole workspace, or only the packages listed in
  `wasm-packages` / `WASM32_PACKAGES`. Use the include list when host-only
  crates (wasmtime, tokio, ...) share the workspace with guest components.
- **Targets** are `--lib --bins --examples` with `--all-features`, then
  `cargo hack clippy --each-feature --exclude-all-features` over cargo's
  default targets (lib + bins). **Tests and benches are never built for
  wasm32**: integration tests and their dev-dependencies are host-only in
  most workspaces, and cargo-hack forwards `--lib`/`--examples` verbatim,
  which errors on packages without those targets.
- **Convention**: any bin or example in the wasm32 scope that is host-only
  must be cfg-gated to an empty `main` on wasm32, e.g.

  ```rust
  cfg_if::cfg_if! {
      if #[cfg(not(target_arch = "wasm32"))] {
          // host-only imports and `main`
      } else {
          fn main() {}
      }
  }
  ```

  Libraries in scope must build for wasm32 with every single feature and with
  all features at once. Integration tests may stay ungated, though workspaces
  that already gate them with `#![cfg(not(target_arch = "wasm32"))]` lose
  nothing.

A workspace with no wasm32 target shadows `lint-wasm` with a local no-op task,
as this repository's own [mise.toml](mise.toml) does.

### Mirror: `ci.yaml` jobs and `mise/rust.toml` tasks

The workflow keeps explicit cargo steps and `mise/rust.toml` mirrors them by
hand. When you change one, change the other in the same PR.

| `ci.yaml` job | `mise/rust.toml` task | Command |
|---|---|---|
| Format | `fmt-check` | `cargo +nightly fmt --all --check` |
| Clippy (host steps) | `lint-host` | `cargo clippy --locked --workspace --all-targets --all-features`, `cargo hack clippy --locked --workspace --each-feature --exclude-all-features` |
| Clippy (per-target step) | `lint-wasm` | `cargo clippy --locked <scope> --lib --bins --examples --all-features --target wasm32-wasip2`, `cargo hack clippy --locked <scope> --each-feature --exclude-all-features --target wasm32-wasip2` |
| Test | `test` | `cargo nextest run --locked --workspace --all-features --no-tests=pass` |
| Test docs | `test-docs` | `cargo test --doc --locked --all-features --workspace`, when the workspace has a library target |
| Docs | `docs` | `cargo doc --no-deps --workspace --all-features --locked` with `RUSTDOCFLAGS=-Dwarnings` |
| Vet | `vet` | `cargo vet --locked` |
| Deny | `deny` | `cargo deny --workspace check` |
| Conventions | `conventions-check` | `conventions check`, the program built from the toolkit revision in use: CI's `job.workflow_sha`, mise's `?ref=` tag |

All clippy/test/doc steps run with `RUSTFLAGS=-Dwarnings` (workflow-global
`env`; per-task `env` in mise). `mise run ci` runs the tasks in the table
order; `lint` runs `lint-host` then `lint-wasm`. Tasks with no CI job
(`audit`, `outdated`, `deps`, `fmt`, `vet-regen`, `conventions-sync`, `cov`,
`publish`, `miri`, `clean`, `sweep`) are local helpers; `audit` and `outdated`
correspond to the scheduled `audit.yaml` workflow instead.

## Conventions

[`conventions/`](conventions) is the tree of files every consumer carries and
[`crates/conventions`](crates/conventions) the program that writes and checks
them. [`conventions/manifest.toml`](conventions/manifest.toml) lists each
managed file with its mode. The program embeds the tree of the release it is
built from, so the conventions a repository carries are those of its pinned
toolkit version; `make conventions-sync` builds it from the tag the `mise.toml`
pin names (into `target/toolkit/`, never committed) and runs `conventions sync`;
`make conventions-check` runs `conventions check` the same way, as `make ci`
and the reusable `ci.yaml` do.

### Modes

| Mode | Files | Ownership |
|---|---|---|
| whole | `rustfmt.toml`, `taplo.toml`, `Makefile`, `LICENSE-MIT`, `LICENSE-APACHE`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`, `renovate.json` | Written verbatim from the tree, opening with the two-line [managed notice](#the-managed-notice) where the format takes comments (the licences and `renovate.json` carry none). A local edit fails `check`; change the tree instead. |
| block | `AGENTS.md`, `CONTRIBUTING.md`, `.gitignore`, `.github/dependabot.yml` | A shared region inside a file the repository owns, from a `BEGIN Managed by augentic/toolkit` line to its `END` line; the repository's own text sits around it. |
| table | `Cargo.toml`, `deny.toml`, `supply-chain/config.toml`, `rust-toolchain.toml` | Shared TOML tables owned by key: every key the tree sets (`[workspace.lints.rust]`, `[licenses] allow`, `[imports.*]`, `[toolchain] channel` and `components`) holds the tree's value, wherever the file keeps it, and every other key of the file (`[toolchain] targets` among them) is the repository's and never moves. A key the file lacks is added beside the shared keys, with its comment from the tree; a key the manifest lists as `retired` is removed. No notice is written into the file; `sync` and `check` print the repository's own keys of each shared table. |
| pin | every `.github/workflows/*.yaml` | Repository-owned; only the `@vX.Y.Z` of each `uses: augentic/toolkit/...` reference is managed. `sync` rewrites it to the pin, `check` compares. |

Everything else is the repository's: `mise.toml` (the pin line apart), the
callers' `with:` inputs, `clippy.toml`, `.vscode/`, `.cargo/`, the vet store
beyond its imports.

### The managed notice

One phrase names everything the toolkit writes: `Managed by augentic/toolkit`,
followed by the path of the source in this repository, then the instruction
on its own line. A whole file opens with the two lines and a blank line:

```toml
# Managed by augentic/toolkit: conventions/rustfmt.toml
# Do not edit: run `make conventions-sync`.

# https://github.com/rust-lang/rustfmt/blob/master/Configurations.md
```

A block carries the same two lines behind `BEGIN` and closes with `END` and
the identifier repeated, so both edges of every block answer
`rg "Managed by augentic/toolkit"` and the closing line stands on its own
when a reader lands on it mid-file:

```markdown
<!-- BEGIN Managed by augentic/toolkit: conventions/agents/git.md -->
<!-- Do not edit: run `make conventions-sync`. -->
## Git
...
<!-- END Managed by augentic/toolkit: conventions/agents/git.md -->
```

```yaml
# BEGIN Managed by augentic/toolkit: conventions/dependabot/actions.yml
# Do not edit: run `make conventions-sync`.
  - package-ecosystem: github-actions
    ...
# END Managed by augentic/toolkit: conventions/dependabot/actions.yml
```

Markdown takes the HTML form; YAML and `.gitignore` the hash form. The name
is the block's source path, `conventions/agents/git.md`, so it is the file
to open here.

`sync` rewrites a block from its `BEGIN` line to its `END` line. A block the
file lacks is appended at the end, in manifest order; a block-mode file the
repository lacks is created with a heading and the blocks. `check` verifies
content and presence, never position, so a block may be moved by hand. A pair
written by 0.3.0 (`conventions:begin agents/git` / `conventions:end
agents/git`, in TOML files too) is read for this release alone: the first
`sync` respells a block's pair in place and drops the pair around a TOML
table, whose keys are owned by value from then on. The one-line header 0.3.0
wrote on `rust-toolchain.toml` and the four caller workflows it rendered is
dropped the same way: those files are the repository's now, held by key and
by pin.

### What `check` enforces

- Every managed file renders byte-equal to the tree; otherwise the unified
  diff is printed and the exit code is 1.
- Every `uses: augentic/toolkit/...@vX.Y.Z` equals the mise `?ref=vX.Y.Z`,
  and the program's own version equals it (the tag it was built from).
- Marker pairs are well-formed, uniquely named, and never nested.
- Every key a shared TOML table sets holds the tree's value, a `retired` key
  is gone, and the file parses as TOML; the repository's other keys are not
  held.
- `AGENTS.md` is at most 30 KiB and carries the shared blocks: agent loaders
  cap what they read (Codex at 32 KiB across the chain), and the Git rule
  must be inside the cap.
- The code of conduct is spelled `CODE_OF_CONDUCT.md`, the one spelling
  GitHub's community profile recognises.

### Developing the tree

This repository carries the conventions it publishes: its `mise.toml` sets
`CONVENTIONS_TOOLKIT = "."`, so `make conventions-sync` and
`make conventions-check` run the working tree's program over the root, and a
change under `conventions/` re-syncs the root in the same commit (`make ci`
says so when it is forgotten; its `conventions` job runs `check` here as in
any consumer). A change is tried against a consumer checkout before it is
tagged:

```shell
cargo run -p conventions -- sync --toolkit . --root ../emery
cargo run -p conventions -- check --toolkit . --root ../emery
```

`--toolkit <dir>` reads `<dir>/conventions` instead of the embedded copy and
takes the program's own version as the pin, so the consumer's `mise.toml` need
not point anywhere yet. From the consumer's side, `CONVENTIONS_TOOLKIT=../toolkit
make conventions-sync` runs the same thing through the mise task. The crate's
tests run `sync` then `check` over a fixture consumer, migrate one 0.3.0 left,
and fail each class of local drift.

## Supply chain

Every consumer imports the same six upstream audit sets, and one more from
this repository:

```toml
[imports.augentic]
url = "https://raw.githubusercontent.com/augentic/toolkit/main/supply-chain/augentic/audits.toml"
```

[`supply-chain/augentic/sources.list`](supply-chain/augentic/sources.list)
names the `audits.toml` of every consumer and of this repository's own
workspace; the scheduled `vet-aggregate.yaml` runs `cargo vet aggregate` over
it and opens a pull request with the result at
`supply-chain/augentic/audits.toml`, the pattern cargo-vet documents for
multiple repositories. An audit certified in one repository
(`cargo vet certify`, a wildcard audit) reaches every other through the
aggregate.

`[[trusted.*]]` publisher entries do not travel: cargo-vet imports audits and
wildcard audits, never another store's trust decisions, so each repository
keeps its own. This repository's
[`supply-chain/audits.toml`](supply-chain/audits.toml) carries the union of
the organisation's trusted publishers as the reference a repository copies an
entry from when it adds a dependency, and the aggregate shows them beside the
audits; renew expiring entries in each repository (`end` a year on) rather
than here.

## Required secrets

Some reusable workflows require secrets to be provisioned by the calling
repository.

### `release.yaml`

Creates a release branch from `main` and opens a "bump version" PR back into
`main`. The PR is opened using a GitHub App installation token so that branch
protection / required-status-checks fire on the PR.

| Secret | Required | Used for |
|---|---|---|
| `APP_ID` | yes | App ID of a GitHub App installed on the repository with `contents: write` and `pull_requests: write` permissions. Passed to `actions/create-github-app-token` as `client-id` (see below). |
| `APP_PRIVATE_KEY` | yes | PEM-encoded private key for the same GitHub App. |
| `CARGO_REGISTRY_TOKEN` | yes | Token used by `cargo update --workspace` when private-registry dependencies are present. |

These secrets are validated before any job runs: a calling repository that
inherits the workflow without `APP_ID` / `APP_PRIVATE_KEY` available fails
immediately with `Secret APP_ID is required, but not provided while calling.`

#### Why a GitHub App token

The "bump version" PR cannot be opened with the default `GITHUB_TOKEN`,
because PRs created by it do not trigger other workflows — so branch
protection and required status checks would never fire and the PR could not be
merged. The workflow instead mints a short-lived installation token from a
GitHub App (`actions/create-github-app-token`), which is treated as a real
actor and lets CI run on the PR. `vet-aggregate.yaml` in this repository opens
its pull request the same way.

#### Setting up the GitHub App

1. Create a GitHub App in the `augentic` org (Settings → Developer settings →
   GitHub Apps → New GitHub App).
2. Grant it the repository permissions **Contents: Read and write** and
   **Pull requests: Read and write**. No webhook is required.
3. Note the **App ID** and generate a **private key** (a `.pem` file).
4. **Install** the App on the org and grant it access to every repository that
   calls `release.yaml`, and to this one.
5. Provision the secrets. Prefer **organization** Actions secrets shared with
   the consuming repositories — the same pattern as `CARGO_REGISTRY_TOKEN` —
   so every release-capable repo inherits them:
   - `APP_ID` — the numeric App ID.
   - `APP_PRIVATE_KEY` — the full PEM key, including the `BEGIN`/`END` lines.

`actions/create-github-app-token` deprecated its `app-id` input in favour of
`client-id`. The workflow passes `APP_ID` as `client-id`: GitHub accepts either
the App ID or the Client ID as the JWT issuer, so the existing secret keeps
working and nothing needs to be re-provisioned. If you would rather store the
App's Client ID (`Iv1...`) in `APP_ID`, that works too.

### `publish.yaml`

No secrets beyond `GITHUB_TOKEN`. The workflow is safe to re-run after a
partial failure (for example when the downstream `crates.yaml` job is rate
limited): the `Release <version>` commit is only made when `RELEASES.md` still
says `Unreleased`, the `v<version>` tag is only pushed when it does not already
exist, and the GitHub release is skipped when one already exists for the tag.
The tag job checks out the head of the release branch rather than the
triggering commit so that a re-run sees the commit pushed by the earlier
attempt.

### `crates.yaml`

| Secret | Required | Used for |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | yes | Authenticates `cargo publish --workspace --locked`. |

Re-runnable: workspace members already on crates.io at the current version are
passed to `cargo publish` as `--exclude`, and when crates.io answers `429 Too
Many Requests` (new crates are admitted slowly, see
[crates.io rate limits](https://crates.io/docs/rate-limits)) the job sleeps
until the time crates.io names and retries, up to 12 attempts. Any other
publish failure fails the job immediately.

### `patch.yaml`

| Secret | Required | Used for |
|---|---|---|
| `CARGO_REGISTRY_TOKEN` | yes | Token used by `cargo update --workspace` when private-registry dependencies are present. |

### `wasm.yaml`

| Secret | Required | Used for |
|---|---|---|
| `AZURE_CLIENT_ID` | yes | OIDC federated identity for the Azure CLI. |
| `AZURE_TENANT_ID` | yes | Azure tenant for the federated identity. |
| `AZURE_SUBSCRIPTION_ID` | yes | Azure subscription containing the storage account and container app. |

### `self-release.yaml`, `self-ci.yaml`

Not reusable; they release and test this repository (see
[Releasing this repository](#releasing-this-repository)). `self-ci.yaml` calls
the reusable `ci.yaml` at the commit under test. No secrets beyond
`GITHUB_TOKEN`.

## Releasing this repository

Maintainers cut a release as follows:

1. Open a PR that adds a new `## X.Y.Z` section at the top of
   [RELEASES.md](RELEASES.md), above the previous version, writes the notes
   for it (`### Added` / `### Changed` / `### Fixed`, and `### Conventions`
   for what `conventions sync` will change in a consumer), and sets the same
   version in [Cargo.toml](Cargo.toml) (`workspace.package.version`, which
   `crates/conventions` inherits). Line 1 of `RELEASES.md` is the version the
   `Release` workflow will tag. Squash-merge the PR.
2. In Actions, run the **Release** workflow (`self-release.yaml`) on `main`.
3. The workflow reads the version from line 1, pushes an annotated `vX.Y.Z`
   tag at the head of `main`, and creates a GitHub release named
   `Release vX.Y.Z` whose body is that section of `RELEASES.md` followed by
   GitHub's generated "What's Changed" list (Dependabot PRs are filtered out
   by [.github/release.yaml](.github/release.yaml)). The release is marked as
   latest and created immutably.

The workflow refuses to run when:

- it is dispatched on a branch other than `main`;
- line 1 of `RELEASES.md` is not exactly `## MAJOR.MINOR.PATCH`;
- the `conventions` crate's version differs from line 1, since the program
  reports that version as the pin it was built for;
- the section under line 1 is empty (write the notes first);
- `vX.Y.Z` is already tagged **and** released, which means line 1 was not
  bumped since the last release.

It is safe to re-run after a partial failure: an existing tag without a
release is reused, and an existing release is skipped.

Unlike `publish.yaml` for the Rust repositories, nothing is committed back:
`main` is governed by the organisation's **Merge** ruleset (PR with review and
signed commits), so there is no `Released <date>` line in `RELEASES.md` and
the date lives on the GitHub release. "Unreleased" is simply a version on
line 1 that has no tag yet.

After tagging, each consumer takes the release through Renovate's grouped pull
request (or by hand: bump the `?ref=` and every `uses:`, run
`make conventions-sync`, commit what changed).

## Composite actions in reusable workflows

A reusable workflow must never reference this repository's composite actions
as `augentic/toolkit/.github/actions/<name>@main`: a consumer pinned to
`@v0.1.0` would still run the actions from `main`. `uses:` cannot take an
expression, so the tag cannot be substituted at release time either.

Instead, each job that needs a composite action checks out this repository at
the commit the reusable workflow itself is running from, using the `job`
context, and references the actions by local path:

```yaml
      - uses: actions/checkout@v7            # consumer repository

      - name: Check out shared actions
        uses: actions/checkout@v7
        with:
          repository: ${{ job.workflow_repository }}
          ref: ${{ job.workflow_sha }}
          path: .augentic
          persist-credentials: false
      - run: echo '/.augentic/' >> .git/info/exclude

      - uses: ./.augentic/.github/actions/git-identity
```

Local `uses:` paths must live inside the workspace, so the checkout lands in
`.augentic/` next to the consumer's code. The `.git/info/exclude` line hides
it from git without touching tracked files, so neither `git commit -am` nor
`peter-evans/create-pull-request` (which stages untracked files by default)
can carry it into a consumer branch. The composite actions run with the
workspace root as their working directory, so `cargo` and `git` still act on
the consumer repository. The `conventions` job of `ci.yaml` uses the same
checkout to build the program at the workflow's own commit.

`job.workflow_repository` / `job.workflow_sha` are newer than the `job`
context type bundled with the pinned actionlint, so
[.github/actionlint.yaml](.github/actionlint.yaml) ignores those two reports
per workflow; add an entry there when a new workflow adopts the pattern.
