# Changelog

## [0.5.0](https://github.com/kimulaco/term-survivors/releases/tag/v0.5.0) - 2026-04-25

### Game updates

- feat: display update notification on title screen (https://github.com/kimulaco/term-survivors/pull/40)

## [0.4.2](https://github.com/kimulaco/term-survivors/releases/tag/v0.4.2) - 2026-04-19

### Game updates

- No gameplay changes in this release
- Update dependencies
  - fix(deps): update rust crate rand to 0.9 [security] (https://github.com/kimulaco/term-survivors/pull/30)
  - fix(deps): update rust crate rand to 0.10 (https://github.com/kimulaco/term-survivors/pull/35)

### Other changes

- chore(deps): update actions/upload-artifact action to v7 (https://github.com/kimulaco/term-survivors/pull/36)
- chore(deps): update github actions (https://github.com/kimulaco/term-survivors/pull/34)

## [0.4.1](https://github.com/kimulaco/term-survivors/releases/tag/v0.4.1) - 2026-04-16

### Game updates

- No gameplay changes in this release

### Other changes

- chore: improvement npm security (https://github.com/kimulaco/term-survivors/pull/26)

## [0.4.0](https://github.com/kimulaco/term-survivors/releases/tag/v0.4.0) - 2026-04-12

### Game updates

- feat: update to final wave more difficult (https://github.com/kimulaco/term-survivors/pull/22)
- feat: add game over effect (https://github.com/kimulaco/term-survivors/pull/21)
- feat: add player damage effect (https://github.com/kimulaco/term-survivors/pull/20)

### Other changes

- doc: update game overview and control guide (https://github.com/kimulaco/term-survivors/pull/23)

## [0.3.0](https://github.com/kimulaco/term-survivors/releases/tag/v0.3.0) - 2026-04-09

### Game updates

- feat: add dark mode setting (https://github.com/kimulaco/term-survivors/pull/14)

### Other changes

- feat: refactor ui theme logic (https://github.com/kimulaco/term-survivors/pull/16)

## [0.2.2](https://github.com/kimulaco/term-survivors/releases/tag/v0.2.2) - 2026-04-08

### Game updates

- feat: update log message format (https://github.com/kimulaco/term-survivors/pull/8)

### Other changes

- fix: deduplicate save directory constant between (https://github.com/kimulaco/term-survivors/pull/9)
- fix: remove redundant existence check (https://github.com/kimulaco/term-survivors/pull/10)

## 0.2.1

skipped (deployment failure)

## [0.2.0](https://github.com/kimulaco/term-survivors/releases/tag/v0.2.0) - 2026-04-07

### Game update

- feat: remove sound feature (https://github.com/kimulaco/term-survivors/pull/4)
  - **Reason for this change:** Implementing this feature in a cross-platform manner is challenging. Additionally, standard sounds like beeps do not fit this game's use case requirements.
- feat: add file logging to `~/.term_survivors/logs/latest.log` (https://github.com/kimulaco/term-survivors/pull/5)

### Other changes

- chore: include README in npm main package (https://github.com/kimulaco/term-survivors/pull/6)
- ci: upload release assets to GitHub release (https://github.com/kimulaco/term-survivors/pull/7)

## [0.1.2](https://github.com/kimulaco/term-survivors/releases/tag/v0.1.2) - 2026-04-05

- Added support for Trusted Publishing for npm deployments. (https://github.com/kimulaco/term-survivors/pull/1)

## [0.1.1](https://github.com/kimulaco/term-survivors/releases/tag/v0.1.1) - 2026-04-05

- Added support for Trusted Publishing for crate.io deployments.

## [0.1.0](https://github.com/kimulaco/term-survivors/releases/tag/v0.1.0) - 2026-04-05

- First beta release
- Terminal-based roguelike shooter — survive 5 minutes of auto-combat against bug swarms and defeat the final boss "Kernel Panic" to clear the game
- Six weapons: Orbit, Laser, Drone, Bomb, Scatter, Thunder — first weapon is randomized each run
- Save and resume: session is automatically saved and can be resumed from the title screen
- `simulate` developer command for headless automated playtesting and difficulty tuning (enabled via `--features simulate`, not included in distributed builds)
