# Development

## Build & Run

```bash
# Run
cargo run

# Build
cargo build --release
```

## Testing

```bash
cargo test
```

## Simulate (Automated Playtest)

`simulate` is a developer-only command. It runs the game headlessly with a scripted bot and collects statistics for difficulty tuning. Not included in distributed builds.

```bash
# Run 4 weapons × 20 games (80 total)
cargo run --features simulate -- simulate

# Specify number of games per weapon
cargo run --features simulate -- simulate --games 100

# Save CSV to file (summary is printed to terminal)
cargo run --features simulate -- simulate --games 100 > results.csv

# Speed up with a release build
cargo run --features simulate --release -- simulate --games 200 > results.csv
```

### Output

**stdout (CSV):**

```
game,starting_weapon,outcome,elapsed_sec,kill_count,final_level,final_hp,weapons
1,Orbit,GameOver,121.1,180,10,0,"Orbit|Drone|Laser"
...
```

**stderr (summary):**

```
=== Summary by starting weapon ===
Weapon    Clear%    AvgSurv(s)    AvgKills
Orbit       0.0%         111         151
...
```

### Reading the Results

**CSV columns:**

| Column | Description |
|---|---|
| `game` | Sequential game number |
| `starting_weapon` | The weapon the bot started with |
| `outcome` | `Cleared` = beat the boss, `GameOver` = died, `Timeout` = hit tick limit without resolution |
| `elapsed_sec` | Survival time in seconds (game time, not wall time) |
| `kill_count` | Total enemies killed |
| `final_level` | Player level at end of game |
| `final_hp` | Remaining HP (0 = dead) |
| `weapons` | All weapons held at end, pipe-separated |

**Summary metrics:**

| Metric | Description |
|---|---|
| `Clear%` | Percentage of games cleared. Target: 20–50%. Too high = too easy; too low = too hard. |
| `AvgSurv(s)` | Average survival time in seconds. The full game is 600s. Useful even when Clear% is 0. |
| `AvgKills` | Average kill count. Low kills with long survival may indicate the bot is surviving by running rather than fighting. |

**Interpreting weapon balance:**

- A weapon with much lower `AvgSurv(s)` than others is underperforming — check damage, range, or fire rate.
- A weapon with `Clear%` far above others is overtuned.
- Compare `final_level` across weapons: if one weapon clears at a significantly lower level, it may be gaining XP less efficiently.

### Difficulty Tuning Workflow

```bash
# Capture baseline before changes
cargo run --features simulate --release -- simulate --games 100 > before.csv

# Edit config/enemy.rs or config/weapon.rs

# Capture results after changes
cargo run --features simulate --release -- simulate --games 100 > after.csv

# Compare AvgSurv(s) and Clear% to assess impact
```
