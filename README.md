# Term Survivors

> **Note:** This project is currently under development. Features and APIs may change without notice.

A Vampire Survivors-like roguelike shooter that runs in the terminal.

## Overview

Survive waves of enemies for 5+ minutes and defeat the final boss to clear the game. Weapons fire automatically — just move and stay alive.

- Platforms: macOS / Linux / Windows

## Play

```bash
npx term-survivors
```

Or install permanently:

```bash
# npm
npm install -g term-survivors

# cargo
cargo install term-survivors
```

Then run:

```bash
term-survivors
```

```bash
term-survivors --help
term-survivors 0.1.2

USAGE:
    term-survivors [COMMAND]

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

COMMANDS:
    start    Start the game [default]
    clear    Delete save data (~/.term_survivors)
```

## Controls

Keyboard only. Mouse is not supported.

| Key | Action |
|-----|--------|
| `W` `A` `S` `D` / Arrow keys | Move |
| `Space` | Pause / resume (during play) |
| `1` `2` `3` | Choose upgrade on level up |
| `Enter` | Start game / resume saved session (title screen) |
| `N` | Start new game (title screen) |
| `M` | Return to title (during play) |
| `R` | Retry after game over / clear |
| `Q` / `ESC` | Quit |

## Save Data

Save data is stored in `~/.term_survivors/`:

To delete all save data:

```bash
term-survivors clear
```

## Development

For contributors: see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)

## License

[MIT](./LICENSE)
