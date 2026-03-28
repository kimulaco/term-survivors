use crate::config;

/// Top-left of a `width`×`height` hitbox centered on the orbit ring (aspect-corrected Y).
pub fn orbit_hitbox_top_left(
    player_x: i32,
    player_y: i32,
    radius: i32,
    angle: f64,
    width: i32,
    height: i32,
) -> (i32, i32) {
    let w = width.max(1);
    let h = height.max(1);
    let cx = player_x as f64 + radius as f64 * angle.cos();
    let cy = player_y as f64 + radius as f64 * angle.sin() * config::TERMINAL_Y_ASPECT;
    (
        (cx - w as f64 / 2.0).floor() as i32,
        (cy - h as f64 / 2.0).floor() as i32,
    )
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Movement {
    Static,
    Linear {
        dx: i32,
        dy: i32,
    },
    Orbit {
        cx: i32,
        cy: i32,
        radius: i32,
        angle: f64,
        speed: f64,
    },
    Homing {
        base_dx: i32,
        base_dy: i32,
        target_id: Option<u64>,
    },
}

#[derive(Clone)]
pub struct Projectile {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub glyph: char,
    pub damage: i32,
    pub ttl: u32,
    pub movement: Movement,
    pub pierce: i32,
    pub knockback: i32,
    /// Index into WEAPON_HIT_COOLDOWNS: Orbit=0, Laser=1, Pulse=2, Drone=3
    pub weapon_kind_idx: u8,
}

impl Projectile {
    pub fn new(
        x: i32,
        y: i32,
        glyph: char,
        damage: i32,
        ttl: u32,
        movement: Movement,
        pierce: i32,
    ) -> Self {
        Self {
            x,
            y,
            width: 1,
            height: 1,
            glyph,
            damage,
            ttl,
            movement,
            pierce,
            knockback: 0,
            weapon_kind_idx: 0,
        }
    }

    pub fn with_size(mut self, width: i32, height: i32) -> Self {
        self.width = width.max(1);
        self.height = height.max(1);
        self
    }

    pub fn with_knockback(mut self, knockback: i32) -> Self {
        self.knockback = knockback;
        self
    }

    pub fn with_weapon_kind(mut self, idx: u8) -> Self {
        self.weapon_kind_idx = idx;
        self
    }

    pub fn update(&mut self, player_x: i32, player_y: i32, enemies: &[(u64, i32, i32)]) {
        if self.ttl == 0 {
            return;
        }
        self.ttl -= 1;

        match &mut self.movement {
            Movement::Static => {}
            Movement::Linear { dx, dy } => {
                self.x += *dx;
                self.y += *dy;
            }
            Movement::Orbit {
                cx: _,
                cy: _,
                radius,
                angle,
                speed,
            } => {
                *angle += *speed;
                let (nx, ny) = orbit_hitbox_top_left(
                    player_x,
                    player_y,
                    *radius,
                    *angle,
                    self.width,
                    self.height,
                );
                self.x = nx;
                self.y = ny;
            }
            Movement::Homing {
                base_dx,
                base_dy,
                target_id,
            } => {
                let target_pos = target_id.and_then(|tid| {
                    enemies
                        .iter()
                        .find(|(id, _, _)| *id == tid)
                        .map(|&(_, x, y)| (x, y))
                });
                if let Some((tx, ty)) = target_pos {
                    let dx = tx - self.x;
                    let dy = ty - self.y;
                    if dx.abs() >= dy.abs() {
                        self.x += dx.signum();
                    } else {
                        self.y += dy.signum();
                    }
                } else {
                    *target_id = None;
                    self.x += *base_dx;
                    self.y += *base_dy;
                }
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        self.ttl == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_correctly() {
        let p = Projectile::new(5, 10, '*', 3, 60, Movement::Static, 1);
        assert_eq!(p.x, 5);
        assert_eq!(p.y, 10);
        assert_eq!(p.width, 1);
        assert_eq!(p.height, 1);
        assert_eq!(p.glyph, '*');
        assert_eq!(p.damage, 3);
        assert_eq!(p.ttl, 60);
        assert_eq!(p.pierce, 1);
    }

    #[test]
    fn is_expired_when_ttl_zero() {
        let p = Projectile::new(0, 0, '.', 1, 0, Movement::Static, 1);
        assert!(p.is_expired());

        let p = Projectile::new(0, 0, '.', 1, 1, Movement::Static, 1);
        assert!(!p.is_expired());
    }

    #[test]
    fn update_static_no_position_change() {
        let mut p = Projectile::new(5, 5, '.', 1, 10, Movement::Static, 1);
        p.update(0, 0, &[]);
        assert_eq!(p.x, 5);
        assert_eq!(p.y, 5);
        assert_eq!(p.ttl, 9);
    }

    #[test]
    fn update_static_does_nothing_when_expired() {
        let mut p = Projectile::new(5, 5, '.', 1, 0, Movement::Static, 1);
        p.update(0, 0, &[]);
        assert_eq!(p.ttl, 0);
    }

    #[test]
    fn update_linear_moves_in_direction() {
        let mut p = Projectile::new(5, 5, '-', 1, 10, Movement::Linear { dx: 1, dy: 0 }, 1);
        p.update(0, 0, &[]);
        assert_eq!(p.x, 6);
        assert_eq!(p.y, 5);
        assert_eq!(p.ttl, 9);

        p.update(0, 0, &[]);
        assert_eq!(p.x, 7);
    }

    #[test]
    fn update_orbit_follows_player() {
        let mut p = Projectile::new(
            0,
            0,
            '*',
            1,
            100,
            Movement::Orbit {
                cx: 0,
                cy: 0,
                radius: 4,
                angle: 0.0,
                speed: 0.1,
            },
            1,
        );
        p.update(10, 10, &[]);
        let dx = (p.x - 10).abs();
        let dy = (p.y - 10).abs();
        assert!(dx <= 5 && dy <= 5, "orbit projectile should be near player");
    }

    #[test]
    fn update_homing_moves_toward_target() {
        let mut p = Projectile::new(
            5,
            5,
            '>',
            1,
            60,
            Movement::Homing {
                base_dx: 1,
                base_dy: 0,
                target_id: Some(1),
            },
            1,
        );
        p.update(0, 0, &[(1, 10, 5)]);
        assert_eq!(p.x, 6);
        assert_eq!(p.y, 5);
    }

    #[test]
    fn update_homing_moves_in_base_direction_without_target() {
        let mut p = Projectile::new(
            5,
            5,
            '>',
            1,
            60,
            Movement::Homing {
                base_dx: 1,
                base_dy: 0,
                target_id: None,
            },
            1,
        );
        p.update(0, 0, &[]);
        assert_eq!(p.x, 6);
        assert_eq!(p.y, 5);
    }

    #[test]
    fn update_homing_falls_back_to_linear_when_target_dead() {
        let mut p = Projectile::new(
            5,
            5,
            '>',
            1,
            60,
            Movement::Homing {
                base_dx: 1,
                base_dy: 0,
                target_id: Some(99),
            },
            1,
        );
        // Target id 99 not in list -> falls back to base direction
        p.update(0, 0, &[(1, 10, 5)]);
        assert_eq!(p.x, 6, "should move in base direction when target is gone");
    }

    #[test]
    fn update_homing_ignores_non_target_enemies() {
        let mut p = Projectile::new(
            5,
            5,
            '>',
            1,
            60,
            Movement::Homing {
                base_dx: 1,
                base_dy: 0,
                target_id: Some(1),
            },
            1,
        );
        // Target id=1 is at (10,5), but id=2 is closer at (6,5) - should still go to id=1
        p.update(0, 0, &[(1, 10, 5), (2, 6, 5)]);
        assert_eq!(p.x, 6);
        assert_eq!(p.y, 5);
    }
}
