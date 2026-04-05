# AGENTS.md

## Project Overview

TERM SURVIVORS is a Vampire Survivors-like roguelike shooter that runs entirely in the terminal. Built with Rust (edition 2021). The player character `@` survives 5 minutes of auto-combat against bug swarms and defeats a final boss ("Kernel Panic") to clear the game.

## Architecture

Single-binary TUI application with a synchronous game loop running at 60 FPS.

### Key Design Patterns

- **Movement accumulator pattern:** Enemies and orbit projectiles use a fractional accumulator (`move_acc`) to convert continuous speed values into discrete cell-based movement at 60 FPS. Player movement is direct (1 cell per key press per frame).
- **Aspect ratio correction:** All circular/orbital calculations multiply the Y component by 0.5 to compensate for terminal cells being roughly twice as tall as they are wide.

## Build & Run

Requires terminal size of at least 80x24.

## Formatting

```bash
cargo fmt
cargo clippy --all-features -- -D warnings
```

Run `cargo fmt` after every code change. The project uses `rustfmt` with default settings. `.vscode/settings.json` is configured to run `rustfmt` on save via `rust-analyzer`.

Run `cargo clippy --all-features -- -D warnings` after every implementation. Warnings are treated as errors.

## Design Philosophy

### Unified stat structs over scattered constants

Group related parameters into a single `pub struct` (e.g., `EnemyStats`, `WeaponStats`) rather than multiple `pub const` values. Standalone constants like `BOSS_WIDTH`, `MIDBOSS_KNOCKBACK_DIVISOR` are a code smell — they should be fields.

Rule: if two or more constants describe the same entity, put them in a struct.

### Enums over bool flags for state

Prefer exhaustive enums over multiple `bool` fields. Two booleans produce 4 combinations but often only 3 are valid — an enum eliminates the impossible state.

```rust
// Bad: boss_spawned: bool + boss_alive: bool (4 combinations, 1 invalid)
// Good:
enum BossState { NotSpawned, Alive, Defeated }
```

The same applies to behavior variants: prefer `SpawnBehavior::Once { spawn_tick }` over `spawn_once_tick: Option<u32>`.

### Enum over trait for closed sets

Use `enum` (not `dyn Trait`) for closed sets of entities (enemy kinds, weapon kinds). Reasons:
- The set of enemies/weapons is fixed per game design
- `dyn Trait` is incompatible with `serde` serialization
- `match` exhaustiveness is a compile-time safety net

Use `trait` only when the set is truly open (e.g., a plugin system).

### Responsibility separation: game state vs. display state

- `GameState` (in `systems::session`) owns game logic: HP, position, XP, kill count, spawn timers.
- `App` (in `systems::state`) owns display/UX state: screen shake ticks, current phase, pending UI events.
- `TickResult { outcome: TickOutcome, screen_shake: u32 }` is the boundary: game logic communicates *what happened*; `App` decides *how to show it*.

Do not put visual/UX fields (`shake_ticks`, `screen_flash`) inside `GameState`.

### Ownership by association, not by convenience

Assign fields to the struct they conceptually belong to:
- Player HP and level → `GameState` (they exist even when the player is not on screen)
- Weapon level → `Weapon` (each weapon instance tracks its own upgrade)
- Screen shake → `App` (pure display effect, not game logic)

When unsure, ask: "if we swapped the renderer, would this field need to change?" If yes, it belongs in `App`/`ui.rs`.

### Config coverage

All tunable numbers live in `src/config/`. No magic numbers in `entities/` or `systems/`. The `config/mod.rs` re-exports everything so `use crate::config::XXX` works from any module.

Split config by domain: `display.rs`, `player.rs`, `enemy.rs`, `weapon.rs`, `game.rs`.

### Per-variant state with enums

When only one variant of an enum needs runtime state, encode it in the enum variant rather than on the parent struct:

```rust
// Bad: orbit_angle: f64 on every Weapon regardless of kind
// Good:
enum WeaponState { Orbit { angle: f64 }, Laser, Pulse, Drone }
```

## Coding Conventions

- Coordinate system: `(0, 0)` is top-left of the game field. X increases rightward, Y increases downward.
- Entity positions are `i32` (not unsigned) to allow temporary out-of-bounds without overflow.
- The game loop is synchronous; avoid introducing async or threading.
- Rendering is strictly separated in `src/ui.rs`. Game logic modules must not depend on ratatui.
- For character-level cell rendering in the arena, use the local `set_cell(buf, x, y, ch, color)` helper in `ui.rs`, which writes directly to a `&mut Buffer` obtained via the `Widget` trait.
- Weapons create `Projectile` instances; they never directly modify enemy state.

## Testing

```bash
cargo test
```

Unit tests are defined as `#[cfg(test)] mod tests` blocks at the bottom of each module file.

Not tested (by design): `main` (terminal I/O entry point), `ui` (ratatui rendering), `config` (constants only).

The `simulate` feature (`cargo build --features simulate`) enables `src/systems/simulate.rs`, a developer-only headless runner used for balance testing. It is excluded from published crate builds via `include` in `Cargo.toml`.

## npm Distribution

The `npm/` directory contains the packages published to npmjs.com:

- `npm/term-survivors/` — main package with a Node.js shim (`index.js`) as the `bin` entry
- `npm/@term-survivors/{platform}/` — platform binary packages (`darwin-arm64`, `darwin-x64`, `linux-x64`, `linux-arm64`, `win32-x64`)

Uses the **optionalDependencies pattern** (same as esbuild / Biome): the main package lists all platform packages as `optionalDependencies`, and npm skips packages whose `os`/`cpu` fields don't match the current platform. The JS shim resolves the correct binary at runtime via `require.resolve`.

Binary artifacts are built by the `build-npm` matrix job in `.github/workflows/publish.yml` and published by the `publish-npm` job. The version in `Cargo.toml` and all `package.json` files must be kept in sync when cutting a release.

Publishing uses npm Trusted Publishing (OIDC); no `NPM_TOKEN` secret is needed. Configure per-package provenance on npmjs.com before the first publish.

## Save Data

Saved to `~/.term_survivors/`. The directory is created automatically on first save. Session data is deleted on game over, clear, or new game start.
