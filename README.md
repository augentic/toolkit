# Toolkit

The shared engineering toolkit of Augentic's Rust repositories: reusable
GitHub workflows, composite actions, and mise tasks. Consumers pin one release
tag (`@vX.Y.Z`) rather than `@main`; see [Versioning](#versioning).

Formerly `augentic/.github`, which now holds the organisation profile and
default community health files alone. Tags up to `v0.2.0` resolve from both
repositories.

## Versioning

This repository is released as `vX.Y.Z` tags with a matching GitHub release.
While on `0.x`, a **minor** bump signals a breaking change (renamed inputs,
changed behaviour, removed workflows, a convention a consumer must act on) and
a **patch** bump is a fix. Release notes live in [RELEASES.md](RELEASES.md).

A consumer carries the tag in two places, every `uses:` and the mise `?ref=`,
and bumps them together:

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
pinning the tag pins everything it runs.

`@main` still works for trying unreleased changes but is not the supported
reference: it can change under a consumer at any time.

## Consumer configuration

A Rust repository adopts the shared tasks and CI with its `mise.toml`, a
`Makefile` forwarding to mise, and its own caller workflows:

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
# Makefile — forwards `make <task> [args]` to `mise run <task> -- [args]`;
# mise is never installed implicitly, see https://mise.jdx.dev/getting-started.html
MISE := $(shell command -v mise 2>/dev/null)

.PHONY: %
%:
	@if [ "$@" = "$(firstword $(MAKECMDGOALS))" ]; then \
		if [ -z "$(MISE)" ]; then \
			echo "mise not found on PATH; install it first: https://mise.jdx.dev/getting-started.html" >&2; \
			exit 1; \
		fi; \
		"$(MISE)" run "$@" -- $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS)); \
	fi
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
chooses. `make ci` runs the same checks as the workflow; `make check` adds the
local-only advisories (`audit`, `outdated`, `deps`) and rewrites formatting in
place.

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

A workspace with no wasm32 target shadows `lint-wasm` with a local no-op task:

```toml
[tasks.lint-wasm]
description = "Nothing to lint for wasm32-wasip2 in this workspace"
run = "true"
```

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

All clippy/test/doc steps run with `RUSTFLAGS=-Dwarnings` (workflow-global
`env`; per-task `env` in mise). `mise run ci` runs the tasks in the table
order; `lint` runs `lint-host` then `lint-wasm`. Tasks with no CI job
(`audit`, `outdated`, `deps`, `fmt`, `vet-regen`, `cov`, `publish`, `miri`,
`clean`, `sweep`) are local helpers; `audit` and `outdated` correspond to the
scheduled `audit.yaml` workflow instead.

## Supply chain

Every consumer imports one audit set from this repository, beside the
upstream ones:

```toml
[imports.augentic]
url = "https://raw.githubusercontent.com/augentic/toolkit/main/supply-chain/augentic/audits.toml"
```

[`supply-chain/augentic/sources.list`](supply-chain/augentic/sources.list)
names the `audits.toml` of every consumer and of this repository; the
scheduled `vet-aggregate.yaml` runs `cargo vet aggregate` over
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

#### Drafted release notes

`release.yaml` and `patch.yaml` draft the new `RELEASES.md` section before
they write it: a `notes` job puts the pull requests merged since the previous
tag (the `(#N)` squash-merge subjects between the tag and `main`, or between
the branch's tag and its head for a patch) to the Copilot CLI through
[`github/copilot-release-notes`](https://github.com/github/copilot-release-notes),
pinned by commit, under the shared style guide
[`.github/release-notes-instructions.md`](.github/release-notes-instructions.md):
one entry per pull request under `Added`, `Changed`, `Fixed`, `Removed`, or
`Security`; version bumps and behaviour-free dependency bumps skipped; an
entry the model is unsure of under `### Needs Review` with its reason. A
repository's own `.github/release-notes-instructions.md` takes the shared
guide's place when it carries one.

Prerequisites:

- The organisation's Copilot plan allows the CLI under a workflow's token
  ("Allow use of Copilot CLI billed to the organization"). The requests are
  billed to the organisation; no personal token is involved.
- The caller's `permissions` carry `copilot-requests: write`. A reusable
  workflow cannot exceed its caller, so each repository's `release.yaml` and
  `patch.yaml` grant it beside `contents: write` (and `pull-requests: write`
  for the release).

The `notes` job holds no secret and writes nothing: `contents: read`,
`pull-requests: read`, `copilot-requests: write`, a checkout without
credentials, and the CLI pinned by `npm install -g @github/copilot@<version>`
before the action runs (Dependabot proposes the action's commit bump; the CLI
pin is bumped by hand beside it). A draft is accepted when it carries at least
one entry and no `## ` heading or `---` rule, and reaches the `create` or
`patch` job as an artifact. When the job fails, the action's five-minute bound
is exceeded, or nothing is drafted (the first release has no previous tag; a
very large range may not finish), the run warns and the section is the list
GitHub generates, as before: nothing blocks the cut.

The draft is reviewed where the notes have always been reviewed: the `Update
release notes for <version>` commit on the release branch, which the "bump
version" pull request links. Resolve every entry under `### Needs Review`
there; `publish.yaml` refuses to date and tag a `RELEASES.md` that still
carries that heading.

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

### `self-release.yaml`

Not reusable; it releases this repository (see
[Releasing this repository](#releasing-this-repository)). No secrets beyond
`GITHUB_TOKEN`.

## Releasing this repository

Maintainers cut a release as follows:

1. Open a PR that adds a new `## X.Y.Z` section at the top of
   [RELEASES.md](RELEASES.md), above the previous version, and writes the
   notes for it (`### Added` / `### Changed` / `### Fixed` / `### Removed`).
   Line 1 of `RELEASES.md` is the version the `Release` workflow will tag.
   Squash-merge the PR.
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

After tagging, each consumer bumps the `?ref=` and every `uses:` to the new
tag.

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
the consumer repository.

`job.workflow_repository` / `job.workflow_sha` are newer than the `job`
context type bundled with the pinned actionlint, so
[.github/actionlint.yaml](.github/actionlint.yaml) ignores those two reports
per workflow; add an entry there when a new workflow adopts the pattern.
