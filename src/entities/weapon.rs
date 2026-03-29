use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::config::{self, WeaponFireConfig};
use crate::entities::projectile::{self, Movement, Projectile};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WeaponKind {
    Orbit,
    Laser,
    Drone,
}

impl WeaponKind {
    pub fn stats(&self) -> &'static config::WeaponStats {
        match self {
            WeaponKind::Orbit => &config::WEAPON_ORBIT,
            WeaponKind::Laser => &config::WEAPON_LASER,
            WeaponKind::Drone => &config::WEAPON_DRONE,
        }
    }

    pub fn name(&self) -> &'static str {
        self.stats().name
    }

    pub fn description(&self) -> &'static str {
        self.stats().description
    }

    /// Index into weapon hit cooldown table (Orbit=0, Laser=1, Drone=2).
    pub fn idx(&self) -> u8 {
        match self {
            WeaponKind::Orbit => 0,
            WeaponKind::Laser => 1,
            WeaponKind::Drone => 2,
        }
    }
}

/// Per-variant runtime state for a weapon instance.
#[derive(Clone, Serialize, Deserialize)]
pub enum WeaponState {
    Orbit { angle: f64 },
    Laser,
    Drone,
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
    ) {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_kind_name_and_description() {
        let kinds = [WeaponKind::Orbit, WeaponKind::Laser, WeaponKind::Drone];
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
        w.update(5, 5, &mut projectiles, &[]);
        assert!(projectiles.is_empty());
        assert_eq!(w.cooldown_timer, 9);
    }

    #[test]
    fn update_fires_when_cooldown_zero() {
        let mut w = Weapon::new(WeaponKind::Laser);
        w.cooldown_timer = 0;
        let mut projectiles = Vec::new();
        w.update(5, 5, &mut projectiles, &[]);
        assert!(!projectiles.is_empty(), "should fire projectiles");
        assert!(w.cooldown_timer > 0, "cooldown should be set after firing");
    }

    #[test]
    fn orbit_fires_projectiles() {
        let mut w = Weapon::new(WeaponKind::Orbit);
        let mut projectiles = Vec::new();
        w.update(10, 10, &mut projectiles, &[]);
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
        w.update(10, 10, &mut projectiles, &[]);
        assert_eq!(projectiles.len(), 1);

        w.level_up();
        w.cooldown_timer = 0;
        projectiles.clear();
        w.update(10, 10, &mut projectiles, &[]);
        assert_eq!(projectiles.len(), 2);
    }
}
