use std::f64::consts::PI;

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::config::{self, WeaponFireConfig};
use crate::entities::player::FacingDir;
use crate::entities::projectile::{self, Movement, Projectile};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WeaponKind {
    Orbit,
    Laser,
    Drone,
    Bomb,
    Scatter,
    Thunder,
}

impl WeaponKind {
    pub fn stats(&self) -> &'static config::WeaponStats {
        match self {
            WeaponKind::Orbit => &config::WEAPON_ORBIT,
            WeaponKind::Laser => &config::WEAPON_LASER,
            WeaponKind::Drone => &config::WEAPON_DRONE,
            WeaponKind::Bomb => &config::WEAPON_BOMB,
            WeaponKind::Scatter => &config::WEAPON_SCATTER,
            WeaponKind::Thunder => &config::WEAPON_THUNDER,
        }
    }

    pub fn name(&self) -> &'static str {
        self.stats().name
    }

    pub fn description(&self) -> &'static str {
        self.stats().description
    }

    pub fn abbr(&self) -> &'static str {
        match self {
            WeaponKind::Orbit => "Or",
            WeaponKind::Laser => "La",
            WeaponKind::Drone => "Dr",
            WeaponKind::Bomb => "Bo",
            WeaponKind::Scatter => "Sc",
            WeaponKind::Thunder => "Th",
        }
    }

    /// Index into weapon hit cooldown table (Orbit=0, Laser=1, Drone=2, Bomb=3, Scatter=4, Thunder=5).
    pub fn idx(&self) -> u8 {
        match self {
            WeaponKind::Orbit => 0,
            WeaponKind::Laser => 1,
            WeaponKind::Drone => 2,
            WeaponKind::Bomb => 3,
            WeaponKind::Scatter => 4,
            WeaponKind::Thunder => 5,
        }
    }

    pub fn from_idx(idx: u8) -> Self {
        match idx {
            0 => WeaponKind::Orbit,
            1 => WeaponKind::Laser,
            2 => WeaponKind::Drone,
            3 => WeaponKind::Bomb,
            4 => WeaponKind::Scatter,
            _ => WeaponKind::Thunder,
        }
    }
}

/// Per-variant runtime state for a weapon instance.
#[derive(Clone, Serialize, Deserialize)]
pub enum WeaponState {
    Orbit {
        angle: f64,
    },
    Laser,
    Drone,
    Bomb {
        /// Ticks remaining until the 2nd bomb is placed (Lv2+). None = no pending bomb.
        #[serde(default)]
        pending_bomb_timer: Option<u32>,
    },
    Scatter,
    Thunder,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub kind: WeaponKind,
    pub level: u32,
    pub cooldown_timer: u32,
    pub state: WeaponState,
}

impl Weapon {
    pub fn new(kind: WeaponKind) -> Self {
        let state = match kind {
            WeaponKind::Orbit => WeaponState::Orbit { angle: 0.0 },
            WeaponKind::Laser => WeaponState::Laser,
            WeaponKind::Drone => WeaponState::Drone,
            WeaponKind::Bomb => WeaponState::Bomb {
                pending_bomb_timer: None,
            },
            WeaponKind::Scatter => WeaponState::Scatter,
            WeaponKind::Thunder => WeaponState::Thunder,
        };
        Self {
            kind,
            level: 1,
            cooldown_timer: 0,
            state,
        }
    }

    pub fn level_up(&mut self) {
        if self.level < 5 {
            self.level += 1;
        }
    }

    pub fn damage(&self) -> i32 {
        self.kind.stats().damage_table[(self.level as usize - 1).min(4)]
    }

    fn cooldown(&self) -> u32 {
        let base = self.kind.stats().cooldown.0;
        let reduction = (self.level - 1) as f64 * 0.1;
        (base as f64 * (1.0 - reduction)).max(5.0) as u32
    }

    pub fn update(
        &mut self,
        player_x: i32,
        player_y: i32,
        projectiles: &mut Vec<Projectile>,
        enemies: &[(u64, i32, i32)],
        facing: FacingDir,
    ) {
        // Lv2+ Bomb: fire the 2nd bomb at the player's current position after stagger delay.
        if let WeaponState::Bomb {
            ref mut pending_bomb_timer,
        } = self.state
        {
            if let Some(ref mut t) = *pending_bomb_timer {
                *t -= 1;
                if *t == 0 {
                    *pending_bomb_timer = None;
                    self.fire_bomb_single(player_x, player_y, projectiles);
                }
            }
        }

        if self.cooldown_timer > 0 {
            self.cooldown_timer -= 1;
            if let (WeaponState::Orbit { angle }, WeaponFireConfig::Orbit { rotation_speed, .. }) =
                (&mut self.state, config::WEAPON_ORBIT.fire)
            {
                *angle += rotation_speed;
            }
            return;
        }

        self.cooldown_timer = self.cooldown();
        match self.kind {
            WeaponKind::Orbit => {
                let angle = if let WeaponState::Orbit { angle } = self.state {
                    angle
                } else {
                    0.0
                };
                self.fire_orbit(player_x, player_y, projectiles, angle);
                if let (
                    WeaponState::Orbit { angle: ref mut a },
                    WeaponFireConfig::Orbit { rotation_speed, .. },
                ) = (&mut self.state, config::WEAPON_ORBIT.fire)
                {
                    *a += rotation_speed;
                }
            }
            WeaponKind::Laser => self.fire_laser(player_x, player_y, projectiles),
            WeaponKind::Drone => self.fire_drone(player_x, player_y, projectiles, enemies),
            WeaponKind::Bomb => {
                self.fire_bomb(player_x, player_y, projectiles);
            }
            WeaponKind::Scatter => self.fire_scatter(player_x, player_y, projectiles, facing),
            WeaponKind::Thunder => self.fire_thunder(player_x, player_y, projectiles, enemies),
        }
    }

    fn fire_orbit(
        &self,
        player_x: i32,
        player_y: i32,
        projectiles: &mut Vec<Projectile>,
        angle: f64,
    ) {
        let WeaponFireConfig::Orbit {
            radius,
            radius_per_level,
            rotation_speed,
            base_count,
            width,
            height,
        } = config::WEAPON_ORBIT.fire
        else {
            return;
        };
        let dmg = self.damage();
        let cooldown = self.cooldown();
        let pierce = self.level as i32;
        let idx = self.kind.idx();
        let effective_radius = radius + self.level as i32 * radius_per_level;
        let count = base_count + self.level as usize;

        for i in 0..count {
            let proj_angle = angle + (i as f64 * 2.0 * PI / count as f64);
            let (px, py) = projectile::orbit_hitbox_top_left(
                player_x,
                player_y,
                effective_radius,
                proj_angle,
                width,
                height,
            );
            projectiles.push(
                Projectile::new(
                    px,
                    py,
                    '*',
                    dmg,
                    cooldown,
                    Movement::Orbit {
                        cx: player_x,
                        cy: player_y,
                        radius: effective_radius,
                        angle: proj_angle,
                        speed: rotation_speed,
                    },
                    pierce,
                )
                .with_size(width, height)
                .with_weapon_kind(idx),
            );
        }
    }

    fn fire_laser(&self, player_x: i32, player_y: i32, projectiles: &mut Vec<Projectile>) {
        let WeaponFireConfig::Laser {
            base_length,
            length_per_level,
            knockback,
        } = config::WEAPON_LASER.fire
        else {
            return;
        };
        let dmg = self.damage();
        let pierce = self.level as i32;
        let idx = self.kind.idx();
        let total_length = base_length + self.level as i32 * length_per_level;

        for &(dx, dy, glyph) in &[(1i32, 0i32, '-'), (-1, 0, '-'), (0, 1, '|'), (0, -1, '|')] {
            let length = if dy != 0 {
                total_length / 2
            } else {
                total_length
            };
            for dist in 1..=length {
                let bx = player_x + dx * dist;
                let by = player_y + dy * dist;
                // Center cell + 2 perpendicular cells
                let offsets: &[(i32, i32)] = if dx != 0 {
                    &[(0, 0), (0, -1), (0, 1)]
                } else {
                    &[(0, 0), (-1, 0), (1, 0)]
                };
                for &(ox, oy) in offsets {
                    projectiles.push(
                        Projectile::new(bx + ox, by + oy, glyph, dmg, 8, Movement::Static, pierce)
                            .with_knockback(knockback)
                            .with_weapon_kind(idx),
                    );
                }
            }
        }
    }

    fn fire_drone(
        &self,
        player_x: i32,
        player_y: i32,
        projectiles: &mut Vec<Projectile>,
        enemies: &[(u64, i32, i32)],
    ) {
        let dmg = self.damage();
        let idx = self.kind.idx();
        let count = self.level as usize;
        let directions: &[(i32, i32)] = &[(1, 0), (-1, 0), (0, 1), (0, -1)];

        let mut sorted_enemies: Vec<(u64, i32, i32)> = enemies.to_vec();
        sorted_enemies.sort_by_key(|&(_, ex, ey)| (ex - player_x).abs() + (ey - player_y).abs());

        for i in 0..count {
            let (base_dx, base_dy, target_id) = if let Some(&(eid, ex, ey)) = sorted_enemies.get(i)
            {
                let dx = ex - player_x;
                let dy = ey - player_y;
                let dir = if dx.abs() >= dy.abs() {
                    (dx.signum(), 0)
                } else {
                    (0, dy.signum())
                };
                (dir.0, dir.1, Some(eid))
            } else {
                let d = directions[i % directions.len()];
                (d.0, d.1, None)
            };
            projectiles.push(
                Projectile::new(
                    player_x,
                    player_y,
                    '>',
                    dmg,
                    90,
                    Movement::Homing {
                        base_dx,
                        base_dy,
                        target_id,
                    },
                    1,
                )
                .with_weapon_kind(idx),
            );
        }
    }

    fn fire_scatter(
        &self,
        player_x: i32,
        player_y: i32,
        projectiles: &mut Vec<Projectile>,
        facing: FacingDir,
    ) {
        let WeaponFireConfig::Scatter { spread, ttl } = config::WEAPON_SCATTER.fire else {
            return;
        };
        let dmg = self.damage();
        let idx = self.kind.idx();
        // Spread grows with level: Lv1→3 wide, Lv3→5 wide, Lv5→7 wide.
        let effective_spread = spread + (self.level as i32 - 1) / 2;

        let (dx, dy) = facing.to_dir();
        let glyph = match (dx, dy) {
            (1, 0) => '>',
            (-1, 0) => '<',
            (0, -1) => '^',
            (0, 1) => 'v',
            _ => '*',
        };
        // Vertical shots travel 2× farther visually per tick due to terminal cell aspect ratio.
        let effective_ttl = if dx == 0 {
            (ttl as f64 * config::TERMINAL_Y_ASPECT).round() as u32
        } else {
            ttl
        };

        for offset in -effective_spread..=effective_spread {
            // Perpendicular offset: horizontal travel → offset Y, vertical travel → offset X.
            let (ox, oy) = if dy == 0 { (0, offset) } else { (offset, 0) };
            projectiles.push(
                Projectile::new(
                    player_x + ox,
                    player_y + oy,
                    glyph,
                    dmg,
                    effective_ttl,
                    Movement::Linear { dx, dy },
                    1,
                )
                .with_weapon_kind(idx),
            );
        }
    }

    fn fire_thunder(
        &self,
        player_x: i32,
        player_y: i32,
        projectiles: &mut Vec<Projectile>,
        enemies: &[(u64, i32, i32)],
    ) {
        let WeaponFireConfig::Thunder {
            warn_ticks,
            warn_reduction_per_level,
            base_jitter,
            base_radius,
        } = config::WEAPON_THUNDER.fire
        else {
            return;
        };

        let dmg = self.damage();
        let idx = self.kind.idx();
        let strike_count = self.level.div_ceil(2) as usize;
        let effective_warn = warn_ticks.saturating_sub((self.level - 1) * warn_reduction_per_level);
        let jitter = (base_jitter - (self.level as i32 - 1) / 2).max(0);
        let radius = base_radius + (self.level as i32 - 1) / 3;

        let mut rng = rand::rng();

        // Sort by distance to player, keep up to 3 closest as target candidates
        let mut sorted = enemies.to_vec();
        sorted.sort_by_key(|&(_, ex, ey)| (ex - player_x).abs() + (ey - player_y).abs());
        let candidates = &sorted[..sorted.len().min(3)];

        for _ in 0..strike_count {
            // Target: random from 3 closest enemies, or random field position if none
            let (base_x, base_y) = if candidates.is_empty() {
                (
                    rng.random_range(0..config::MAX_FIELD_WIDTH),
                    rng.random_range(0..config::MAX_FIELD_HEIGHT),
                )
            } else {
                let (_, ex, ey) = candidates[rng.random_range(0..candidates.len())];
                (ex, ey)
            };
            let tx = base_x
                + if jitter > 0 {
                    rng.random_range(-jitter..=jitter)
                } else {
                    0
                };
            let ty = base_y
                + if jitter > 0 {
                    rng.random_range(-jitter..=jitter)
                } else {
                    0
                };

            // Warning indicator (0 damage, visible for warn_ticks)
            projectiles.push(
                Projectile::new(tx, ty, '!', 0, effective_warn + 1, Movement::Static, 1)
                    .with_weapon_kind(idx),
            );

            // Lightning strike: filled ellipse (aspect-ratio corrected), delayed
            let r = radius as f64;
            for dy in -radius..=radius {
                let half_w = ((r * r - (dy as f64 * dy as f64)).max(0.0).sqrt() * 2.0) as i32;
                for dx in -half_w..=half_w {
                    let adx = dx as f64 * 0.5;
                    let ady = dy as f64;
                    if adx * adx + ady * ady <= r * r {
                        projectiles.push(
                            Projectile::new(tx + dx, ty + dy, '#', dmg, 4, Movement::Static, 1)
                                .with_delay(effective_warn)
                                .with_weapon_kind(idx),
                        );
                    }
                }
            }
        }
    }

    /// Place one bomb at the given position using the current level's fuse duration.
    fn fire_bomb_single(&self, x: i32, y: i32, projectiles: &mut Vec<Projectile>) {
        let WeaponFireConfig::Bomb {
            radius,
            fuse_ticks,
            fuse_reduction_per_level,
            ..
        } = config::WEAPON_BOMB.fire
        else {
            return;
        };
        let dmg = self.damage();
        let idx = self.kind.idx();
        let fuse = fuse_ticks.saturating_sub((self.level - 1) * fuse_reduction_per_level);

        // Fuse indicator (visual only, pierce=1 to survive retain check)
        projectiles.push(
            Projectile::new(x, y, 'o', 0, fuse + 1, Movement::Static, 1).with_weapon_kind(idx),
        );

        // Explosion cells: filled ellipse (aspect-ratio corrected)
        let r = radius as f64;
        for dy in -radius..=radius {
            let half_w = ((r * r - (dy as f64 * dy as f64)).max(0.0).sqrt() * 2.0) as i32;
            for dx in -half_w..=half_w {
                let adx = dx as f64 * 0.5;
                let ady = dy as f64;
                if adx * adx + ady * ady <= r * r {
                    projectiles.push(
                        Projectile::new(x + dx, y + dy, '*', dmg, 4, Movement::Static, 1)
                            .with_delay(fuse)
                            .with_weapon_kind(idx),
                    );
                }
            }
        }
    }

    fn fire_bomb(&mut self, player_x: i32, player_y: i32, projectiles: &mut Vec<Projectile>) {
        self.fire_bomb_single(player_x, player_y, projectiles);

        // Lv2+: schedule a 2nd bomb to drop at the player's future position.
        if self.level >= 2 {
            let WeaponFireConfig::Bomb { stagger_ticks, .. } = config::WEAPON_BOMB.fire else {
                return;
            };
            if let WeaponState::Bomb {
                ref mut pending_bomb_timer,
            } = self.state
            {
                *pending_bomb_timer = Some(stagger_ticks);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_kind_name_and_description() {
        let kinds = [
            WeaponKind::Orbit,
            WeaponKind::Laser,
            WeaponKind::Drone,
            WeaponKind::Bomb,
        ];
        for kind in &kinds {
            assert!(!kind.name().is_empty());
            assert!(!kind.description().is_empty());
        }
    }

    #[test]
    fn new_weapon_defaults() {
        let w = Weapon::new(WeaponKind::Laser);
        assert_eq!(w.kind, WeaponKind::Laser);
        assert_eq!(w.level, 1);
        assert_eq!(w.cooldown_timer, 0);
    }

    #[test]
    fn level_up_increases_level() {
        let mut w = Weapon::new(WeaponKind::Orbit);
        w.level_up();
        assert_eq!(w.level, 2);
    }

    #[test]
    fn level_up_capped_at_five() {
        let mut w = Weapon::new(WeaponKind::Orbit);
        for _ in 0..10 {
            w.level_up();
        }
        assert_eq!(w.level, 5);
    }

    #[test]
    fn damage_scales_with_level() {
        let mut w = Weapon::new(WeaponKind::Laser);
        let dmg1 = w.damage();
        w.level_up();
        let dmg2 = w.damage();
        assert!(dmg2 > dmg1, "damage should increase with level");
    }

    #[test]
    fn update_does_not_fire_during_cooldown() {
        let mut w = Weapon::new(WeaponKind::Laser);
        w.cooldown_timer = 10;
        let mut projectiles = Vec::new();
        w.update(5, 5, &mut projectiles, &[], FacingDir::Right);
        assert!(projectiles.is_empty());
        assert_eq!(w.cooldown_timer, 9);
    }

    #[test]
    fn update_fires_when_cooldown_zero() {
        let mut w = Weapon::new(WeaponKind::Laser);
        w.cooldown_timer = 0;
        let mut projectiles = Vec::new();
        w.update(5, 5, &mut projectiles, &[], FacingDir::Right);
        assert!(!projectiles.is_empty(), "should fire projectiles");
        assert!(w.cooldown_timer > 0, "cooldown should be set after firing");
    }

    #[test]
    fn orbit_fires_projectiles() {
        let mut w = Weapon::new(WeaponKind::Orbit);
        let mut projectiles = Vec::new();
        w.update(10, 10, &mut projectiles, &[], FacingDir::Right);
        let (base_count, width, height) = if let WeaponFireConfig::Orbit {
            base_count,
            width,
            height,
            ..
        } = config::WEAPON_ORBIT.fire
        {
            (base_count, width, height)
        } else {
            panic!("unexpected fire config");
        };
        assert_eq!(projectiles.len(), base_count + 1); // Lv1: base_count + level
        assert_eq!(projectiles[0].width, width);
        assert_eq!(projectiles[0].height, height);
    }

    #[test]
    fn drone_fires_correct_count() {
        let mut w = Weapon::new(WeaponKind::Drone);
        let mut projectiles = Vec::new();
        w.update(10, 10, &mut projectiles, &[], FacingDir::Right);
        assert_eq!(projectiles.len(), 1);

        w.level_up();
        w.cooldown_timer = 0;
        projectiles.clear();
        w.update(10, 10, &mut projectiles, &[], FacingDir::Right);
        assert_eq!(projectiles.len(), 2);
    }

    #[test]
    fn bomb_fires_fuse_and_explosion_projectiles() {
        let mut w = Weapon::new(WeaponKind::Bomb);
        let mut projectiles = Vec::new();
        w.update(10, 10, &mut projectiles, &[], FacingDir::Right);
        // 1 fuse indicator + N explosion cells
        let fuse_count = projectiles.iter().filter(|p| p.glyph == 'o').count();
        let explosion_count = projectiles.iter().filter(|p| p.glyph == '*').count();
        assert_eq!(fuse_count, 1, "Lv1: 1 bomb = 1 fuse indicator");
        assert!(explosion_count > 0, "should have explosion cells");
        // All explosion cells must have delay_ticks > 0
        assert!(
            projectiles
                .iter()
                .filter(|p| p.glyph == '*')
                .all(|p| p.delay_ticks > 0),
            "explosion cells must start delayed"
        );
    }

    #[test]
    fn bomb_spawns_one_at_lv1() {
        let mut w = Weapon::new(WeaponKind::Bomb);
        let mut projectiles = Vec::new();
        w.fire_bomb(10, 10, &mut projectiles);
        let fuse_count = projectiles.iter().filter(|p| p.glyph == 'o').count();
        assert_eq!(fuse_count, 1, "Lv1 should spawn 1 bomb immediately");
    }

    #[test]
    fn bomb_lv2_fires_first_bomb_immediately() {
        let mut w = Weapon::new(WeaponKind::Bomb);
        w.level = 2;
        let mut projectiles = Vec::new();
        w.fire_bomb(10, 10, &mut projectiles);
        let fuse_count = projectiles.iter().filter(|p| p.glyph == 'o').count();
        assert_eq!(fuse_count, 1, "Lv2 fires 1st bomb immediately");
    }

    #[test]
    fn bomb_lv2_fires_second_bomb_after_stagger() {
        let WeaponFireConfig::Bomb { stagger_ticks, .. } = config::WEAPON_BOMB.fire else {
            panic!("unexpected fire config");
        };
        let mut w = Weapon::new(WeaponKind::Bomb);
        w.level = 2;
        let mut projectiles = Vec::new();
        // Fire (places 1st bomb + schedules 2nd)
        w.fire_bomb(10, 10, &mut projectiles);
        // Advance stagger_ticks via update (cooldown_timer > 0 so no new fire)
        w.cooldown_timer = stagger_ticks + 10;
        for _ in 0..stagger_ticks {
            w.update(20, 20, &mut projectiles, &[], FacingDir::Right);
        }
        let fuse_count = projectiles.iter().filter(|p| p.glyph == 'o').count();
        assert_eq!(
            fuse_count, 2,
            "2nd bomb should be placed after stagger delay"
        );
        // 2nd bomb is placed at the player's new position (20, 20)
        let second_fuse = projectiles
            .iter()
            .find(|p| p.glyph == 'o' && p.x == 20 && p.y == 20);
        assert!(
            second_fuse.is_some(),
            "2nd bomb should be at new player position"
        );
    }

    #[test]
    fn thunder_fires_randomly_when_no_enemies() {
        let mut w = Weapon::new(WeaponKind::Thunder);
        let mut projectiles = Vec::new();
        w.update(5, 5, &mut projectiles, &[], FacingDir::Right);
        assert!(!projectiles.is_empty(), "should fire even with no enemies");
    }

    #[test]
    fn thunder_fires_warning_and_strike_projectiles() {
        let mut w = Weapon::new(WeaponKind::Thunder);
        let mut projectiles = Vec::new();
        let enemies = vec![(1u64, 10i32, 10i32)];
        w.update(5, 5, &mut projectiles, &enemies, FacingDir::Right);
        let warn_count = projectiles
            .iter()
            .filter(|p| p.glyph == '!' && p.damage == 0)
            .count();
        let strike_count = projectiles
            .iter()
            .filter(|p| p.glyph == '#' && p.damage > 0)
            .count();
        assert_eq!(warn_count, 1, "Lv1 should produce 1 warning indicator");
        assert!(strike_count > 0, "Lv1 should produce strike cells");
        assert!(
            projectiles
                .iter()
                .filter(|p| p.glyph == '#')
                .all(|p| p.delay_ticks > 0),
            "strike cells must start delayed"
        );
    }

    #[test]
    fn thunder_strike_count_scales_with_level() {
        let enemies = vec![(1u64, 10i32, 10i32), (2, 20, 20), (3, 30, 30)];
        for (level, expected_warns) in [(1u32, 1usize), (3, 2), (5, 3)] {
            let mut w = Weapon::new(WeaponKind::Thunder);
            w.level = level;
            let mut projectiles = Vec::new();
            w.fire_thunder(5, 5, &mut projectiles, &enemies);
            let warn_count = projectiles.iter().filter(|p| p.damage == 0).count();
            assert_eq!(
                warn_count, expected_warns,
                "Lv{} should have {} warning indicator(s)",
                level, expected_warns
            );
        }
    }
}
