# Release notes instructions

The notes are the `RELEASES.md` section of one release, read by the users of
the crate or binary the repository ships. Each pull request between the two
refs becomes at most one entry.

## Categories

Tag every entry with exactly one of these category names, spelled as written.
The tag is the heading the entry is listed under.

- `Added`: a capability, verb, flag, input, output, or document that did not
  exist before.
- `Changed`: an existing behaviour that works differently now, including a
  renamed or re-shaped surface.
- `Fixed`: something that was wrong and works now. Describe what works now,
  not what was broken.
- `Removed`: a capability, verb, flag, input, or output that is gone.
- `Security`: a vulnerability closed or a trust boundary tightened.

## What to skip

Do not write an entry for:

- a version bump pull request (`Bump to X.Y.Z`, `Increment Cargo version`)
- a dependency bump, unless it changes behaviour or fixes a vulnerability
- a change with no effect a user can see: CI, tests, benchmarks, internal
  refactoring, comments, formatting

Report every skipped pull request with its reason. Never drop one silently.

## Style

- One or two sentences per entry, present tense, imperative mood ("Add",
  "Refuse", "Read").
- State the effect for the user: what they can do, what they see, what they
  must change. Not the mechanics of the change.
- Names of commands, verbs, flags, files, types, and fields in backticks.
- No author attribution. The pull request number is the trace.

## Uncertainty

When the category or the user-visible effect is unclear from the title, the
body, and the diff, mark the entry uncertain and say why, so a maintainer
reviews it before the release is published.
