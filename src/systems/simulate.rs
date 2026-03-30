use crate::entities::weapon::WeaponKind;
use crate::systems::levelup::Upgrade;
use crate::systems::session::{GameState, TickOutcome};

/// Returns the center of the nearest active bomb fuse (damage=0, weapon_kind_idx=3),
/// or None if no bomb is currently live.
fn active_bomb_center(gs: &GameState) -> Option<(f64, f64)> {
    let bomb_idx = WeaponKind::Bomb.idx();
    gs.projectiles
        .iter()
        .filter(|p| p.damage == 0 && p.weapon_kind_idx == bomb_idx)
        .map(|p| (p.x as f64, p.y as f64))
        .next()
}

const MAX_TICKS: u32 = 120_000;

pub struct RunConfig {
    pub games_per_weapon: usize,
    pub field_width: i32,
    pub field_height: i32,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            games_per_weapon: 20,
            field_width: 78,
            field_height: 22,
        }
    }
}

pub enum RunOutcome {
    GameOver,
    Cleared,
    Timeout,
}

pub struct RunResult {
    pub starting_weapon: WeaponKind,
    pub outcome: RunOutcome,
    pub elapsed_ticks: u32,
    pub kill_count: u32,
    pub final_level: u32,
    pub final_hp: i32,
    pub weapons: Vec<WeaponKind>,
}

/// Score-based upgrade selection.
///
/// Scoring rationale:
/// - Heal: value scales with missing HP; dominates when HP < 50%
/// - LevelUp: worth more for low-level weapons (diminishing returns at high level)
/// - NewWeapon: fixed value; beaten by LevelUp when a weapon is still at Lv1-2
/// - MaxHpUp: always the lowest priority
fn ai_choose_upgrade(choices: &[Upgrade], gs: &GameState) -> usize {
    let hp_ratio = gs.player.hp as f64 / gs.player.max_hp as f64;

    let mut best_idx = 0;
    let mut best_score = f64::NEG_INFINITY;

    for (i, choice) in choices.iter().enumerate() {
        let score: f64 = match choice {
            Upgrade::HealHp => (1.0 - hp_ratio) * 100.0,
            Upgrade::LevelUpWeapon(idx) => {
                let level = gs.weapons.get(*idx).map(|w| w.level).unwrap_or(5);
                // Lv1→2: 60, Lv2→3: 50, Lv3→4: 40, Lv4→5: 30
                70.0 - level as f64 * 10.0
            }
            Upgrade::NewWeapon(_) => 45.0,
            Upgrade::MaxHpUp => 15.0,
        };
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }

    best_idx
}

/// Force-field movement: sum repulsion vectors from all nearby enemies
/// plus a soft wall-avoidance force, then snap to -1/0/1.
///
/// Better than "flee nearest only" because:
/// - avoids being squeezed between two enemies approaching from opposite sides
/// - avoids cornering against the wall
fn ai_move(gs: &GameState) -> (i32, i32) {
    let px = gs.player.x as f64;
    let py = gs.player.y as f64;
    let fw = gs.field_width as f64;
    let fh = gs.field_height as f64;
    let has_laser = gs.weapons.iter().any(|w| w.kind == WeaponKind::Laser);
    let has_bomb = gs.weapons.iter().any(|w| w.kind == WeaponKind::Bomb);
    let has_scatter = gs.weapons.iter().any(|w| w.kind == WeaponKind::Scatter);

    let mut fx = 0.0f64;
    let mut fy = 0.0f64;

    // Enemy repulsion: weight = damage / distance², aspect-ratio corrected
    for e in &gs.enemies {
        let dx = px - e.x as f64;
        let dy = py - e.y as f64;
        let adx = dx;
        let ady = dy * 0.5; // terminal cells are ~2x taller than wide
        let dist_sq = (adx * adx + ady * ady).max(0.5);
        let weight = e.damage as f64 / dist_sq;
        fx += dx * weight;
        fy += dy * weight;

        // Laser axis alignment: when at safe distance, attract toward same row/column.
        // Aligning X (same column) lets the vertical laser beam hit.
        // Aligning Y (same row)    lets the horizontal laser beam hit.
        // Choose whichever axis needs the smaller adjustment.
        if has_laser {
            let dist = dist_sq.sqrt();
            if dist > 8.0 && dist < 30.0 {
                let align_strength = 0.3;
                if dx.abs() < dy.abs() {
                    fx -= dx * align_strength; // pull toward enemy's X
                } else {
                    fy -= dy * align_strength; // pull toward enemy's Y
                }
            }
        }
    }

    // Bomb luring: when a bomb is live, ignore distant enemies so the player
    // stays near the blast zone rather than fleeing far away.
    // Only enemies within the danger threshold contribute repulsion.
    // (Already accumulated above; zero out contributions from distant enemies.)
    if has_bomb && active_bomb_center(gs).is_some() {
        // Recompute fx/fy considering only close enemies
        let mut fx2 = 0.0f64;
        let mut fy2 = 0.0f64;
        let danger_dist_sq = 25.0; // 5-cell radius
        for e in &gs.enemies {
            let dx = px - e.x as f64;
            let dy = py - e.y as f64;
            let adx = dx;
            let ady = dy * 0.5;
            let dist_sq = (adx * adx + ady * ady).max(0.5);
            if dist_sq < danger_dist_sq {
                let weight = e.damage as f64 / dist_sq;
                fx2 += dx * weight;
                fy2 += dy * weight;
            }
        }
        fx = fx2;
        fy = fy2;
    }

    // Scatter standoff: move toward the nearest enemy when too far away so that
    // the player's facing direction points at enemies and shots connect.
    // - Beyond engage_dist: attraction pulls player toward enemy (sets FacingDir).
    // - Within safe_dist: repulsion already handles escape; no attraction.
    if has_scatter {
        let scatter_safe_dist = 7.0_f64;
        let scatter_engage_dist = 14.0_f64;

        if let Some(nearest) = gs.enemies.iter().min_by_key(|e| {
            let dx = e.x as f64 - px;
            let dy = (e.y as f64 - py) * 0.5;
            ((dx * dx + dy * dy) * 1000.0) as i64
        }) {
            let edx = nearest.x as f64 - px;
            let edy = nearest.y as f64 - py;
            let adx = edx;
            let ady = edy * 0.5;
            let dist = (adx * adx + ady * ady).sqrt().max(0.1);

            if dist > scatter_safe_dist {
                // Attract strength scales from 0 at safe_dist up to 8 at engage_dist.
                let t = ((dist - scatter_safe_dist) / (scatter_engage_dist - scatter_safe_dist))
                    .min(1.0);
                let attract = t * 8.0;
                fx += edx / dist * attract;
                fy += edy / dist * attract;
            }
        }
    }

    // Soft wall repulsion: push away from edges within a margin
    let wall_margin = 6.0;
    let wall_strength = 4.0;
    fx += (wall_margin - px).max(0.0) * wall_strength;
    fx -= (px - (fw - 1.0 - wall_margin)).max(0.0) * wall_strength;
    fy += (wall_margin - py).max(0.0) * wall_strength;
    fy -= (py - (fh - 1.0 - wall_margin)).max(0.0) * wall_strength;

    let dx = if fx.abs() > 0.1 {
        fx.signum() as i32
    } else {
        0
    };
    let dy = if fy.abs() > 0.1 {
        fy.signum() as i32
    } else {
        0
    };

    // No force: keep moving to avoid getting stuck.
    // Scatter: drift toward the nearest enemy so FacingDir stays updated.
    // Others: drift toward center.
    if dx == 0 && dy == 0 {
        if has_scatter {
            if let Some(nearest) = gs.enemies.iter().min_by_key(|e| {
                let dx = e.x as f64 - px;
                let dy = (e.y as f64 - py) * 0.5;
                ((dx * dx + dy * dy) * 1000.0) as i64
            }) {
                return (
                    (nearest.x - gs.player.x).signum(),
                    (nearest.y - gs.player.y).signum(),
                );
            }
        }
        return (
            (gs.field_width / 2 - gs.player.x).signum(),
            (gs.field_height / 2 - gs.player.y).signum(),
        );
    }

    (dx, dy)
}

pub fn run_single(cfg: &RunConfig, starting_weapon: WeaponKind) -> RunResult {
    let mut gs = GameState::new(cfg.field_width, cfg.field_height);
    gs.add_weapon(starting_weapon);

    loop {
        if gs.elapsed_ticks >= MAX_TICKS {
            return RunResult {
                starting_weapon,
                outcome: RunOutcome::Timeout,
                elapsed_ticks: gs.elapsed_ticks,
                kill_count: gs.kill_count,
                final_level: gs.level,
                final_hp: gs.player.hp,
                weapons: gs.weapons.iter().map(|w| w.kind).collect(),
            };
        }

        let (dx, dy) = ai_move(&gs);
        let result = gs.tick(dx, dy, false);

        match result.outcome {
            TickOutcome::Continue => {}
            TickOutcome::LevelUp(choices) => {
                if !choices.is_empty() {
                    let idx = ai_choose_upgrade(&choices, &gs);
                    let upgrade = choices[idx.min(choices.len() - 1)];
                    gs.apply_upgrade(upgrade);
                }
            }
            TickOutcome::GameOver => {
                return RunResult {
                    starting_weapon,
                    outcome: RunOutcome::GameOver,
                    elapsed_ticks: gs.elapsed_ticks,
                    kill_count: gs.kill_count,
                    final_level: gs.level,
                    final_hp: gs.player.hp,
                    weapons: gs.weapons.iter().map(|w| w.kind).collect(),
                };
            }
            TickOutcome::Cleared => {
                return RunResult {
                    starting_weapon,
                    outcome: RunOutcome::Cleared,
                    elapsed_ticks: gs.elapsed_ticks,
                    kill_count: gs.kill_count,
                    final_level: gs.level,
                    final_hp: gs.player.hp,
                    weapons: gs.weapons.iter().map(|w| w.kind).collect(),
                };
            }
        }
    }
}

pub const ALL_WEAPONS: [WeaponKind; 6] = [
    WeaponKind::Orbit,
    WeaponKind::Laser,
    WeaponKind::Drone,
    WeaponKind::Bomb,
    WeaponKind::Scatter,
    WeaponKind::Thunder,
];

pub fn run_all(cfg: &RunConfig) -> Vec<RunResult> {
    let mut results = Vec::new();
    for &weapon in &ALL_WEAPONS {
        for _ in 0..cfg.games_per_weapon {
            results.push(run_single(cfg, weapon));
        }
    }
    results
}
