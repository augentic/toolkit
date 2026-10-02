# Cargo Vet

The `cargo vet` store for this repository's own workspace. After a dependency
change, run `make vet-regen` to refresh the imports, exemptions, and unpublished
entries, then `make vet` checks them as CI does.

See the [Cargo Vet book](https://mozilla.github.io/cargo-vet/commands.html) for
more information.
