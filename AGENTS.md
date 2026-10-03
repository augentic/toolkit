# Agent Instructions

## Git

Never `git commit`, `git push`, open or close a pull request, or delete a branch — in this repository or in any sibling checkout — unless the maintainer lifts this for the session, explicitly and for named work. Leave every change uncommitted in the working tree; the maintainer reviews and commits. No plan or to-do list carries a commit, push, or PR step, and an instruction to complete every step does not override this.

## Commands

The repository is reusable GitHub workflows, composite actions, and the shared mise tasks of [`mise/rust.toml`](mise/rust.toml); there is nothing to build here. The one check is `lint.yaml`: [actionlint](https://github.com/rhysd/actionlint) over `.github/workflows/` and `.github/actions/`. Run `actionlint -color` from the repository root before handing over; if it cannot run, say exactly why. The workflows and tasks are exercised by the consuming repositories, not here.
