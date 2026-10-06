## Agent skills

### Issue tracker

Issues are tracked as GitHub Issues on `jpierre-7/all-in` via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Commits and pull requests

No session links and no Claude attribution lines (`Co-Authored-By`, `Claude-Session`, "Generated with Claude Code") in commit messages or PR descriptions.

Never commit build output. Before every commit, check `git status` and `git diff --cached --stat`, and stage files by name rather than `git add -A` or `git add .`. In October 2026 a second cargo build in a top-level `target-base/` folder (3,244 files, about 2 GB) was committed and merged in PR #164, and `main` had to be rewritten to remove it.

## Builds and worktrees

- Cargo builds go in `target/`, which is gitignored. If you need a second build, such as a `main` baseline for the balance sim, set `CARGO_TARGET_DIR` to a folder under `target/` or outside the repo. Never create a new top-level build folder.
- When several agents work at once, each works in its own worktree at `../all-in-<ticket>`, on a new branch from `origin/main`, and leaves the main checkout on `main` and untouched. Remove temporary worktrees when you're done.
