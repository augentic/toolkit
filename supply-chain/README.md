# Cargo Vet

The `cargo vet` store for this repository's own workspace. After a dependency
change, run `make vet-regen` to refresh the imports, exemptions, and unpublished
entries, then `make vet` checks them as CI does.

`audits.toml` also carries the union of the organisation's `[[trusted.*]]`
publisher entries, the reference a consumer copies from: cargo-vet does not
import trust decisions, so each repository keeps its own.

`augentic/` is the organisation's aggregate, not part of this store:
`sources.list` names every consumer's `audits.toml` and this one, and the
scheduled `vet-aggregate.yaml` writes `cargo vet aggregate` over it to
`augentic/audits.toml`, which every consumer imports as `[imports.augentic]`.

See the [Cargo Vet book](https://mozilla.github.io/cargo-vet/commands.html) for
more information.
