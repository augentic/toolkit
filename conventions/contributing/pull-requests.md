## Pull request procedure

Pull requests should be targeted at the `main` branch. Before creating a pull request, go through this checklist:

1. Create a feature branch off of `main`.
2. [Rebase](https://git-scm.com/book/en/Git-Branching-Rebasing) your local changes against `main`.
3. Run `make ci` and confirm that it passes: exactly the CI jobs, in order.
4. Accept the Developer's Certificate of Origin on all commits (see above).

All contributions are made via pull request. All patches from all contributors get reviewed. At least one review from a maintainer is required for all patches (even patches from maintainers). When CI fails, authors are expected to update the pull request until it passes.

Normally, all pull requests must include tests that cover your change. Occasionally, a change will be very difficult to test for; in those cases, include a note in your commit message explaining why.
