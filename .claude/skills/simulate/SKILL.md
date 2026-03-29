---
name: simulate
description: |
  Skill for verifying difficulty and weapon balance in TERM SURVIVORS via automated playtesting.
  Use whenever the user says `/simulate`, "run simulate", "check balance", "how's the difficulty?", or asks about weapon performance.
  Also use when the user consults about a specific weapon (e.g. "Laser feels weak", "enemies are too strong") — run simulate first, then answer based on the results.
user-invocable: true
allowed-tools: Bash(cargo run *)
---

## Purpose

Run the game headlessly with a scripted bot, collect statistics, and share a summary with the developer.
If the user has a question or consultation, interpret the results in that context.

---

## Step 1: Run simulate

```bash
cargo run --features simulate --release -- simulate --games 20 2>&1
```

- stdout: per-game CSV data
- stderr: per-weapon summary table
- Default: 20 games × 4 weapons = 80 total. Use `--games 50` or more for higher confidence.

---

## Step 2: Interpret the results

### Summary metrics

| Metric | Description |
|---|---|
| `Clear%` | Boss kill rate. **Target: 20–50%.** Above = too easy; below = too hard. |
| `AvgSurv(s)` | Average survival time in seconds. Full game = 600s; boss spawns at 270s (4:30). |
| `AvgKills` | Average kill count. High survival with low kills may mean the bot is running without engaging. |

### CSV columns

| Column | Description |
|---|---|
| `outcome` | `Cleared` = boss defeated / `GameOver` = player died / `Timeout` = hit 33-minute tick limit (abnormal) |
| `elapsed_sec` | Survival time in seconds |
| `kill_count` | Total enemies killed |
| `final_level` | Player level at end of run |
| `final_hp` | Remaining HP (0 = dead) |
| `weapons` | Weapons held at end, pipe-separated |

### Weapon balance criteria

- **AvgSurv(s) more than 30% below other weapons** → underperforming. Review `damage_table` or `cooldown` in `config/weapon.rs`.
- **Clear% significantly above other weapons** → overpowered. Same file.
- **final_level notably lower than others** → poor XP efficiency (too hard to kill enemies, low kill count).
- **Multiple Timeouts** → possible game loop issue. Investigate.

### Bot characteristics and result interpretation

**This bot uses fixed heuristic logic, not human judgment.** Results should be read as a lower bound on what a human player can achieve.

Key limitations vs. a human player:

- **Movement**: Sums repulsion vectors from all enemies + soft wall avoidance, then snaps to ±1 per axis. It avoids enemies but cannot read attack patterns or time dodges.
- **Attack efficiency**: A human can position to land hits while evading. The bot cannot do both simultaneously — it often retreats without maximising damage output. This means weapons that require deliberate positioning (Laser, Pulse) will appear weaker in simulation than they are in human hands.
- **Laser**: Has axis-alignment attraction logic to align with enemy rows/columns, but is still far less effective at it than a human who can read the battlefield.
- **Upgrade selection**: Score-based (Heal when low HP → weapon LevelUp → new weapon → MaxHpUp). No adaptive strategy.

**Practical implication**: Use simulate for **relative comparisons between weapons** and to detect large balance gaps, not as an absolute measure of game difficulty. A weapon with low simulate scores might still feel fine for a human player.

### Tuning workflow

```bash
# Capture baseline before changes
cargo run --features simulate --release -- simulate --games 50 > before.csv

# Edit config/weapon.rs or config/enemy.rs

# Capture results after changes
cargo run --features simulate --release -- simulate --games 50 > after.csv

# Compare AvgSurv(s) and Clear% to assess impact
```

**Files to tune:**

- `src/config/weapon.rs` — `damage_table` (per-level damage), `cooldown` (fire interval), range/radius, etc.
- `src/config/enemy.rs` — `hp`, `speed`, `damage`, `xp_value`, `SPAWN_TABLE` (spawn interval and enemy types by time)

---

## Step 3: Report to the developer

Use this format:

```
=== Simulate Results Summary ===

[paste the summary table as-is]

--- Observations ---
- [Overall difficulty: how Clear% compares to the 20–50% target]
- [Weapon balance: flag any underperforming or overpowered weapons]
- [Any notable trends worth mentioning]
```

If the user has a question or consultation, add the answer after the summary, grounded in the results.
If there is no question, share the summary only and invite further discussion.
