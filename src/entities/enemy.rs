use crate::config;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyKind {
    Bug,
    Virus,
    Crash,
    MemLeak,
    Elite,
    MidBoss,
    Boss,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Enemy {
    #[serde(default)]
    pub id: u64,
    pub kind: EnemyKind,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub speed: f64,
    pub damage: i32,
    pub xp_value: u32,
    pub glyph: char,
    pub shake_power: u32,
    move_acc: f64,
    pub width: i32,
    pub height: i32,
    pub knockback_divisor: i32,
    #[serde(default)]
    pub knockback_ticks: u32,
    #[serde(default)]
    pub knockback_dx: i32,
    #[serde(default)]
    pub knockback_dy: i32,
    /// Per-weapon-kind hit cooldown ticks [Orbit, Laser, Drone, Bomb]
    #[serde(default)]
    pub hit_cooldowns: [u32; 4],
}

impl Enemy {
    pub fn new(kind: EnemyKind, x: i32, y: i32) -> Self {
        let stats = match kind {
            EnemyKind::Bug => &config::ENEMY_BUG,
            EnemyKind::Virus => &config::ENEMY_VIRUS,
            EnemyKind::Crash => &config::ENEMY_CRASH,
            EnemyKind::MemLeak => &config::ENEMY_MEMLEAK,
            EnemyKind::Elite => &config::ENEMY_ELITE,
            EnemyKind::MidBoss => &config::ENEMY_MIDBOSS,
            EnemyKind::Boss => &config::ENEMY_BOSS,
        };
        Self {
            id: 0,
            kind,
            x,
            y,
            hp: stats.hp,
            max_hp: stats.hp,
            speed: stats.speed,
            damage: stats.damage,
            xp_value: stats.xp_value,
            glyph: stats.glyph,
            shake_power: stats.shake_power,
            move_acc: 0.0,
            width: stats.width,
            height: stats.height,
            knockback_divisor: stats.knockback_divisor,
            knockback_ticks: 0,
            knockback_dx: 0,
            knockback_dy: 0,
            hit_cooldowns: [0; 4],
        }
    }

    /// Apply damage from a weapon. Returns true if damage landed, false if blocked by hit cooldown.
    pub fn take_damage(&mut self, damage: i32, weapon_kind_idx: usize) -> bool {
        if self.hit_cooldowns[weapon_kind_idx] > 0 {
            return false;
        }
        self.hp -= damage;
        self.hit_cooldowns[weapon_kind_idx] = config::weapon_hit_cooldown(weapon_kind_idx);
        true
    }

    pub fn update(&mut self, player_x: i32, player_y: i32) {
        for cd in &mut self.hit_cooldowns {
            if *cd > 0 {
                *cd -= 1;
            }
        }
        if self.knockback_ticks > 0 {
            self.x += self.knockback_dx;
            self.y += self.knockback_dy;
            self.knockback_ticks -= 1;
            return;
        }

        let dx = player_x - self.x;
        let dy = player_y - self.y;

        self.move_acc += self.speed / config::FPS as f64;
        if self.move_acc >= 1.0 {
            let steps = self.move_acc as i32;
            self.move_acc -= steps as f64;
            for _ in 0..steps {
                if dx.abs() >= dy.abs() {
                    self.x += dx.signum();
                } else {
                    self.y += dy.signum();
                }
            }
        }
    }

    pub fn apply_knockback(&mut self, player_x: i32, player_y: i32, distance: i32) {
        let effective_distance = (distance / self.knockback_divisor).max(1);
        let dx = self.x - player_x;
        let dy = self.y - player_y;
        if dx == 0 && dy == 0 {
            self.knockback_dx = 0;
            self.knockback_dy = -1;
        } else if dx.abs() >= dy.abs() {
            self.knockback_dx = dx.signum();
            self.knockback_dy = 0;
        } else {
            self.knockback_dx = 0;
            self.knockback_dy = dy.signum();
        }
        self.knockback_ticks = effective_distance as u32;
    }

    #[allow(dead_code)]
    pub fn name(&self) -> &'static str {
        let stats = match self.kind {
            EnemyKind::Bug => &config::ENEMY_BUG,
            EnemyKind::Virus => &config::ENEMY_VIRUS,
            EnemyKind::Crash => &config::ENEMY_CRASH,
            EnemyKind::MemLeak => &config::ENEMY_MEMLEAK,
            EnemyKind::Elite => &config::ENEMY_ELITE,
            EnemyKind::MidBoss => &config::ENEMY_MIDBOSS,
            EnemyKind::Boss => &config::ENEMY_BOSS,
        };
        stats.name
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    /// Check if this enemy occupies the given position
    pub fn occupies(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Check if this enemy collides with the player at (px, py)
    pub fn collides_with_player(&self, px: i32, py: i32) -> bool {
        // For multi-cell enemies, check if player is adjacent or overlapping
        px >= self.x - 1
            && px <= self.x + self.width
            && py >= self.y - 1
            && py <= self.y + self.height
    }
}

pub fn spawn_enemy(
    elapsed_ticks: u32,
    spawn_timer: &mut u32,
    field_width: i32,
    field_height: i32,
    boss_spawned: bool,
) -> Option<Enemy> {
    // Find current spawn config
    let mut interval = 90u32;
    let mut kinds_mask = 0b00001u8;
    for &(threshold, intv, mask) in config::SPAWN_TABLE.iter().rev() {
        if elapsed_ticks >= threshold {
            interval = intv;
            kinds_mask = mask;
            break;
        }
    }

    *spawn_timer += 1;
    if *spawn_timer < interval {
        return None;
    }
    *spawn_timer = 0;

    // Final boss spawn
    if let config::SpawnBehavior::Once { spawn_tick } = config::ENEMY_BOSS.spawn {
        if elapsed_ticks >= spawn_tick && !boss_spawned {
            return Some(Enemy::new(
                EnemyKind::Boss,
                field_width / 2 - config::ENEMY_BOSS.width / 2,
                0,
            ));
        }
    }

    let mut rng = rand::thread_rng();

    // Pick random kind from available
    let available: Vec<EnemyKind> = [
        (0b00001, EnemyKind::Bug),
        (0b00010, EnemyKind::Virus),
        (0b00100, EnemyKind::Crash),
        (0b01000, EnemyKind::MemLeak),
        (0b10000, EnemyKind::Elite),
    ]
    .iter()
    .filter(|(bit, _)| (kinds_mask & bit) != 0)
    .map(|(_, kind)| *kind)
    .collect();

    if available.is_empty() {
        return None;
    }

    let kind = available[rng.gen_range(0..available.len())];

    // Spawn at random edge
    let (x, y) = match rng.gen_range(0..4) {
        0 => (rng.gen_range(0..field_width), 0), // top
        1 => (rng.gen_range(0..field_width), field_height - 1), // bottom
        2 => (0, rng.gen_range(0..field_height)), // left
        _ => (field_width - 1, rng.gen_range(0..field_height)), // right
    };

    Some(Enemy::new(kind, x, y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bug_stats() {
        let e = Enemy::new(EnemyKind::Bug, 5, 10);
        assert_eq!(e.x, 5);
        assert_eq!(e.y, 10);
        assert_eq!(e.hp, config::ENEMY_BUG.hp);
        assert_eq!(e.damage, config::ENEMY_BUG.damage);
        assert_eq!(e.xp_value, config::ENEMY_BUG.xp_value);
        assert_eq!(e.glyph, config::ENEMY_BUG.glyph);
        assert_eq!(e.width, 1);
        assert_eq!(e.height, 1);
    }

    #[test]
    fn new_midboss_stats() {
        let e = Enemy::new(EnemyKind::MidBoss, 0, 0);
        assert_eq!(e.hp, config::ENEMY_MIDBOSS.hp);
        assert_eq!(e.damage, config::ENEMY_MIDBOSS.damage);
        assert_eq!(e.width, config::ENEMY_MIDBOSS.width);
        assert_eq!(e.height, config::ENEMY_MIDBOSS.height);
        assert_eq!(e.glyph, config::ENEMY_MIDBOSS.glyph);
    }

    #[test]
    fn new_boss_stats() {
        let e = Enemy::new(EnemyKind::Boss, 0, 0);
        assert_eq!(e.hp, config::ENEMY_BOSS.hp);
        assert_eq!(e.damage, config::ENEMY_BOSS.damage);
        assert_eq!(e.width, config::ENEMY_BOSS.width);
        assert_eq!(e.height, config::ENEMY_BOSS.height);
    }

    #[test]
    fn update_moves_toward_player() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        for _ in 0..config::FPS {
            e.update(10, 0);
        }
        assert!(e.x > 0, "enemy should have moved toward player");
    }

    #[test]
    fn is_dead_when_hp_zero() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        assert!(!e.is_dead());
        e.hp = 0;
        assert!(e.is_dead());
        e.hp = -1;
        assert!(e.is_dead());
    }

    #[test]
    fn occupies_single_cell() {
        let e = Enemy::new(EnemyKind::Bug, 5, 5);
        assert!(e.occupies(5, 5));
        assert!(!e.occupies(6, 5));
        assert!(!e.occupies(4, 5));
    }

    #[test]
    fn occupies_multi_cell_boss() {
        let e = Enemy::new(EnemyKind::Boss, 10, 10);
        assert!(e.occupies(10, 10));
        assert!(e.occupies(
            10 + config::ENEMY_BOSS.width - 1,
            10 + config::ENEMY_BOSS.height - 1
        ));
        assert!(!e.occupies(10 + config::ENEMY_BOSS.width, 10));
    }

    #[test]
    fn collides_with_player_adjacent() {
        let e = Enemy::new(EnemyKind::Bug, 5, 5);
        assert!(e.collides_with_player(5, 5));
        assert!(e.collides_with_player(4, 5));
        assert!(e.collides_with_player(6, 5));
        assert!(e.collides_with_player(5, 4));
        assert!(e.collides_with_player(5, 6));
        assert!(!e.collides_with_player(3, 5));
        assert!(!e.collides_with_player(7, 5));
    }

    #[test]
    fn spawn_enemy_respects_timer() {
        let mut timer = 0u32;
        for _ in 0..59 {
            let result = spawn_enemy(0, &mut timer, 80, 24, false);
            assert!(result.is_none());
        }
        let result = spawn_enemy(0, &mut timer, 80, 24, false);
        assert!(result.is_some());
        assert_eq!(timer, 0);
    }

    #[test]
    fn take_damage_reduces_hp_and_returns_true() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        let initial_hp = e.hp;
        let hit = e.take_damage(10, 0);
        assert!(hit);
        assert_eq!(e.hp, initial_hp - 10);
    }

    #[test]
    fn take_damage_blocked_when_cooldown_active() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        let initial_hp = e.hp;
        e.take_damage(10, 0);
        // Second hit in same weapon slot should be blocked
        let hit = e.take_damage(10, 0);
        assert!(!hit);
        assert_eq!(e.hp, initial_hp - 10);
    }

    #[test]
    fn take_damage_sets_hit_cooldown() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        assert_eq!(e.hit_cooldowns[0], 0);
        e.take_damage(10, 0);
        assert_eq!(e.hit_cooldowns[0], config::weapon_hit_cooldown(0));
    }

    #[test]
    fn take_damage_cooldown_decrements_on_update() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        e.take_damage(10, 0);
        let cd = e.hit_cooldowns[0];
        e.update(0, 0);
        assert_eq!(e.hit_cooldowns[0], cd - 1);
    }

    #[test]
    fn take_damage_independent_cooldowns_per_weapon() {
        let mut e = Enemy::new(EnemyKind::Bug, 0, 0);
        let initial_hp = e.hp;
        // Hit with weapon 0
        e.take_damage(10, 0);
        // Weapon 1 should still land
        let hit = e.take_damage(10, 1);
        assert!(hit);
        assert_eq!(e.hp, initial_hp - 20);
    }

    #[test]
    fn apply_knockback_sets_direction_and_ticks() {
        let mut e = Enemy::new(EnemyKind::Bug, 10, 5);
        e.apply_knockback(5, 5, 3);
        assert_eq!(e.knockback_ticks, 3);
        assert_eq!(e.knockback_dx, 1);
        assert_eq!(e.knockback_dy, 0);
    }

    #[test]
    fn update_processes_knockback_before_normal_move() {
        let mut e = Enemy::new(EnemyKind::Bug, 10, 5);
        e.apply_knockback(5, 5, 2);
        let x_before = e.x;
        e.update(5, 5);
        assert_eq!(e.x, x_before + 1, "should move in knockback direction");
        assert_eq!(e.knockback_ticks, 1);
        e.update(5, 5);
        assert_eq!(e.knockback_ticks, 0);
    }
}
