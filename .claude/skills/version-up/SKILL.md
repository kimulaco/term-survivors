---
name: version-up
description: |
  Bump the game version and populate CHANGELOG.md from GitHub milestone PRs.
  Use when the user says `/version-up`, `/version-up X.Y.Z`, "bump version", "release X.Y.Z", "次のバージョン", "バージョンアップ", or similar. Always use this skill for version bumping.
user-invocable: true
allowed-tools: Bash(sh scripts/bump-version.sh *), Bash(grep *), Bash(gh pr list *), Edit
---

## Purpose

Bump the game version across all files and populate the CHANGELOG.md entry from merged PRs in the corresponding GitHub milestone.

---

## Step 1: Determine the target version

The user invokes this as `/version-up X.Y.Z` or `/version-up`.

**If a version is provided:**
1. Validate the format: must match `X.Y.Z` (digits only, three parts). If invalid → warn and stop.
2. Read the current version from `Cargo.toml`:
   ```bash
   grep '^version = ' Cargo.toml | head -1
   ```
3. If the new version is not strictly greater than the current version → warn and stop.

**If no version is provided:**
1. Read the current version from `Cargo.toml`.
2. Propose a patch bump (e.g., `0.2.1` → `0.2.2`) with the exact command the user would confirm.
3. **Do not proceed until the user confirms.**

---

## Step 2: Run the version bump script

```bash
sh scripts/bump-version.sh X.Y.Z
```

This script:
- Updates `Cargo.toml` and all `npm/*/package.json` files
- Updates the version string in `README.md`
- Inserts a placeholder section at the top of `CHANGELOG.md`:
  ```
  ## [X.Y.Z](https://github.com/kimulaco/term-survivors/releases/tag/vX.Y.Z) - YYYY-MM-DD

  - 
  ```

---

## Step 3: Fetch milestone PRs

Use the `gh` CLI to list merged PRs in the milestone:

```bash
gh pr list --repo kimulaco/term-survivors --milestone "vX.Y.Z" --state merged --json number,title,url
```

For each PR, collect:
- PR number (`number`)
- PR title (`title`)
- PR URL (`url`)

If no PRs are found, note this and skip to Step 5 (leave CHANGELOG placeholder as-is with a comment to the developer).

---

## Step 4: Classify PRs into two groups

**Game updates** — changes a player would notice while playing:
- Gameplay mechanics, enemies, weapons, player stats, in-game visuals
- Save/resume behavior, in-game UI
- Bug fixes that affect the player experience

**Other changes** — tooling, infrastructure, internal code:
- CI/CD workflows, release automation, packaging
- npm distribution, build scripts
- Internal refactoring, code deduplication, constant cleanup
- Documentation

Use the conventional commit prefix as the primary signal:
- `feat:`, `fix:` → default to **Game updates**, but move to Other changes if the description is about CI, packaging, internal structure, or code quality (not player-facing)
- `ci:`, `chore:`, `docs:`, `build:`, `refactor:`, `perf:`, `test:` → **Other changes**

Examples:
- `feat: add new enemy SpikeBot` → Game updates
- `fix: crash on level-up screen` → Game updates
- `fix: deduplicate save directory constant` → Other changes (internal code quality)
- `fix: remove redundant existence check` → Other changes (internal code quality)
- `feat: update log message format` → Game updates (log output is player-visible in `~/.term_survivors/logs/`)
- `ci: upload release assets` → Other changes

---

## Step 5: Write the CHANGELOG entry

Replace the placeholder `- ` in the newly added CHANGELOG.md section with the classified list.

Format:

```markdown
### Game updates

- feat: short description (https://github.com/kimulaco/term-survivors/pull/N)
- fix: short description (https://github.com/kimulaco/term-survivors/pull/N)

### Other changes

- ci: short description (https://github.com/kimulaco/term-survivors/pull/N)
- chore: short description (https://github.com/kimulaco/term-survivors/pull/N)
```

Rules:
- Use the PR title as the changelog line text (strip trailing punctuation if any)
- Omit a section entirely if it has no PRs
- If both sections are empty (no PRs found), leave a `<!-- TODO: fill in changelog -->` comment and tell the developer

Use the `Edit` tool to replace the placeholder in `CHANGELOG.md`. The placeholder to find is:

```
- 
```

(a hyphen followed by a space and a newline, under the new version header)

---

## Step 6: Report to the developer

Show the generated CHANGELOG entry and confirm:
- Which files were updated by `bump-version.sh`
- The PR list used for the changelog, with their classification

Ask the developer to review the CHANGELOG entry and let them know they can edit it before committing.
