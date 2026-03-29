use std::cmp::Reverse;

use serde::{Deserialize, Serialize};

use crate::config;
use crate::entities::enemy::{self, Enemy, EnemyKind};
use crate::entities::player::Player;
use crate::entities::projectile::Projectile;
use crate::entities::weapon::{Weapon, WeaponKind};
use crate::save::GameSaveData;
use crate::systems::combat;
use crate::systems::levelup::{self, Upgrade};

#[derive(Clone, Copy, PartialEq)]
pub enum GameMode {
    Normal,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum BossState {
    #[default]
    NotSpawned,
    Alive,
    Defeated,
}

pub struct GameState {
    #[allow(dead_code)]
    pub mode: GameMode,
    pub player: Player,
    pub enemies: Vec<Enemy>,
    pub projectiles: Vec<Projectile>,
    pub weapons: Vec<Weapon>,
    pub elapsed_ticks: u32,
    pub kill_count: u32,
    pub xp: u32,
    pub level: u32,
    pub(crate) spawn_timer: u32,
    pub(crate) mid_boss_spawn_timer: u32,
    pub(crate) boss_state: BossState,
    pub field_width: i32,
    pub field_height: i32,
    next_enemy_id: u64,
    enemy_pos_buf: Vec<(u64, i32, i32)>,
    projectile_buf: Vec<Projectile>,
}

impl GameState {
    pub fn new(field_width: i32, field_height: i32) -> Self {
        let player = Player::new(field_width / 2, field_height / 2);
        Self {
            mode: GameMode::Normal,
            player,
            enemies: Vec::new(),
            projectiles: Vec::new(),
            weapons: Vec::new(),
            elapsed_ticks: 0,
            kill_count: 0,
            xp: 0,
            level: 1,
            spawn_timer: 0,
            mid_boss_spawn_timer: 0,
            boss_state: BossState::NotSpawned,
            field_width,
            field_height,
            next_enemy_id: 1,
            enemy_pos_buf: Vec::new(),
            projectile_buf: Vec::new(),
        }
    }

    pub fn add_weapon(&mut self, kind: WeaponKind) {
        self.weapons.push(Weapon::new(kind));
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.field_width = width;
        self.field_height = height;
        // Clamp player position
        self.player.x = self.player.x.clamp(0, width - 1);
        self.player.y = self.player.y.clamp(0, height - 1);
    }

    pub fn tick(&mut self, dx: i32, dy: i32, sound_enabled: bool) -> TickResult {
        self.elapsed_ticks += 1;

        // Move player
        self.player
            .update(dx, dy, self.field_width, self.field_height);

        // Spawn regular enemies (includes Boss)
        if let Some(mut enemy) = enemy::spawn_enemy(
            self.elapsed_ticks,
            &mut self.spawn_timer,
            self.field_width,
            self.field_height,
            self.boss_state != BossState::NotSpawned,
        ) {
            enemy.id = self.next_enemy_id;
            self.next_enemy_id += 1;
            if enemy.kind == EnemyKind::Boss {
                self.boss_state = BossState::Alive;
            }
            self.enemies.push(enemy);
        }

        // Spawn MidBoss periodically until Boss arrives
        if let config::SpawnBehavior::Periodic {
            first_tick,
            interval_ticks,
            max_alive,
        } = config::ENEMY_MIDBOSS.spawn
        {
            if self.elapsed_ticks >= first_tick && self.boss_state == BossState::NotSpawned {
                self.mid_boss_spawn_timer += 1;
                let mid_boss_alive = self
                    .enemies
                    .iter()
                    .filter(|e| e.kind == EnemyKind::MidBoss)
                    .count();
                if self.mid_boss_spawn_timer >= interval_ticks && mid_boss_alive < max_alive {
                    self.mid_boss_spawn_timer = 0;
                    let mut mb = Enemy::new(
                        EnemyKind::MidBoss,
                        self.field_width / 2 - config::ENEMY_MIDBOSS.width / 2,
                        0,
                    );
                    mb.id = self.next_enemy_id;
                    self.next_enemy_id += 1;
                    self.enemies.push(mb);
                }
            }
        }

        // Cap enemy count
        if self.enemies.len() > config::MAX_ENEMY_COUNT {
            // Remove farthest enemies
            let px = self.player.x;
            let py = self.player.y;
            self.enemies
                .sort_by_key(|e| Reverse((e.x - px).abs() + (e.y - py).abs()));
            self.enemies.truncate(config::MAX_ENEMY_COUNT);
        }

        // Move enemies
        let px = self.player.x;
        let py = self.player.y;
        for enemy in &mut self.enemies {
            enemy.update(px, py);
        }

        // Build enemy positions for weapon targeting and projectile updates
        self.enemy_pos_buf.clear();
        self.enemy_pos_buf
            .extend(self.enemies.iter().map(|e| (e.id, e.x, e.y)));

        // Fire weapons
        self.projectile_buf.clear();
        let facing = self.player.facing;
        for weapon in &mut self.weapons {
            weapon.update(
                self.player.x,
                self.player.y,
                &mut self.projectile_buf,
                &self.enemy_pos_buf,
                facing,
            );
        }
        self.projectiles.append(&mut self.projectile_buf);
        for proj in &mut self.projectiles {
            proj.update(self.player.x, self.player.y, &self.enemy_pos_buf);
        }

        // Combat: projectiles vs enemies
        let result = combat::process_combat(
            &mut self.projectiles,
            &mut self.enemies,
            self.player.x,
            self.player.y,
        );
        self.xp += result.xp_gained;
        self.kill_count += result.kills;

        // Check if boss was killed
        if self.boss_state == BossState::Alive
            && !self.enemies.iter().any(|e| e.kind == EnemyKind::Boss)
        {
            self.boss_state = BossState::Defeated;
            return TickResult {
                outcome: TickOutcome::Cleared,
                screen_shake: 0,
            };
        }

        // Enemy-player contact
        let screen_shake =
            combat::process_enemy_contact(&self.enemies, &mut self.player, sound_enabled);

        // Clean up expired projectiles
        self.projectiles.retain(|p| !p.is_expired() && p.pierce > 0);

        // Clamp projectiles to field (remove out-of-bounds)
        self.projectiles.retain(|p| {
            let w = p.width.max(1);
            let h = p.height.max(1);
            p.x < self.field_width + 5
                && p.x + w > -5
                && p.y < self.field_height + 5
                && p.y + h > -5
        });

        // Check death
        if self.player.is_dead() {
            return TickResult {
                outcome: TickOutcome::GameOver,
                screen_shake,
            };
        }

        // Check level up
        let threshold = self.xp_threshold();
        if self.xp >= threshold {
            self.xp -= threshold;
            self.level += 1;
            return TickResult {
                outcome: TickOutcome::LevelUp(levelup::generate_choices(&self.weapons)),
                screen_shake,
            };
        }

        TickResult {
            outcome: TickOutcome::Continue,
            screen_shake,
        }
    }

    pub fn xp_threshold(&self) -> u32 {
        let idx = (self.level as usize - 1).min(config::XP_THRESHOLDS.len() - 1);
        config::XP_THRESHOLDS[idx]
    }

    pub fn apply_upgrade(&mut self, upgrade: Upgrade) {
        levelup::apply_upgrade(upgrade, &mut self.weapons, &mut self.player);
    }

    pub fn to_save_data(&self, pending_upgrades: Vec<Upgrade>) -> GameSaveData {
        GameSaveData {
            player: self.player.clone(),
            enemies: self.enemies.clone(),
            weapons: self.weapons.clone(),
            elapsed_ticks: self.elapsed_ticks,
            kill_count: self.kill_count,
            xp: self.xp,
            level: self.level,
            spawn_timer: self.spawn_timer,
            mid_boss_spawn_timer: self.mid_boss_spawn_timer,
            boss_state: self.boss_state,
            field_width: self.field_width,
            field_height: self.field_height,
            pending_upgrades,
        }
    }

    pub fn from_save_data(data: GameSaveData) -> Self {
        let next_enemy_id = data.enemies.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        Self {
            mode: GameMode::Normal,
            player: data.player,
            enemies: data.enemies,
            projectiles: Vec::new(),
            weapons: data.weapons,
            elapsed_ticks: data.elapsed_ticks,
            kill_count: data.kill_count,
            xp: data.xp,
            level: data.level,
            spawn_timer: data.spawn_timer,
            mid_boss_spawn_timer: data.mid_boss_spawn_timer,
            boss_state: data.boss_state,
            field_width: data.field_width,
            field_height: data.field_height,
            next_enemy_id,
            enemy_pos_buf: Vec::new(),
            projectile_buf: Vec::new(),
        }
    }
}

pub struct TickResult {
    pub outcome: TickOutcome,
    pub screen_shake: u32,
}

pub enum TickOutcome {
    Continue,
    LevelUp(Vec<Upgrade>),
    GameOver,
    Cleared,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_state_defaults() {
        let gs = GameState::new(80, 24);
        assert_eq!(gs.field_width, 80);
        assert_eq!(gs.field_height, 24);
        assert_eq!(gs.player.x, 40);
        assert_eq!(gs.player.y, 12);
        assert!(gs.enemies.is_empty());
        assert!(gs.projectiles.is_empty());
        assert!(gs.weapons.is_empty());
        assert_eq!(gs.elapsed_ticks, 0);
        assert_eq!(gs.kill_count, 0);
        assert_eq!(gs.xp, 0);
        assert_eq!(gs.level, 1);
    }

    #[test]
    fn add_weapon_pushes_weapon() {
        let mut gs = GameState::new(80, 24);
        gs.add_weapon(WeaponKind::Orbit);
        assert_eq!(gs.weapons.len(), 1);
        assert_eq!(gs.weapons[0].kind, WeaponKind::Orbit);

        gs.add_weapon(WeaponKind::Laser);
        assert_eq!(gs.weapons.len(), 2);
    }

    #[test]
    fn resize_updates_field_and_clamps_player() {
        let mut gs = GameState::new(80, 24);
        gs.player.x = 79;
        gs.player.y = 23;
        gs.resize(40, 10);
        assert_eq!(gs.field_width, 40);
        assert_eq!(gs.field_height, 10);
        assert_eq!(gs.player.x, 39);
        assert_eq!(gs.player.y, 9);
    }

    #[test]
    fn xp_threshold_level_1() {
        let gs = GameState::new(80, 24);
        assert_eq!(gs.xp_threshold(), config::XP_THRESHOLDS[0]);
    }

    #[test]
    fn xp_threshold_clamped_beyond_table() {
        let mut gs = GameState::new(80, 24);
        gs.level = 100;
        assert_eq!(gs.xp_threshold(), *config::XP_THRESHOLDS.last().unwrap());
    }

    #[test]
    fn tick_increments_elapsed() {
        let mut gs = GameState::new(80, 24);
        gs.add_weapon(WeaponKind::Orbit);
        let _ = gs.tick(0, 0, false);
        assert_eq!(gs.elapsed_ticks, 1);
    }

    #[test]
    fn tick_moves_player() {
        let mut gs = GameState::new(80, 24);
        gs.add_weapon(WeaponKind::Orbit);
        let start_x = gs.player.x;
        let _ = gs.tick(1, 0, false);
        assert_eq!(gs.player.x, start_x + 1);
    }

    #[test]
    fn tick_returns_game_over_when_dead() {
        let mut gs = GameState::new(80, 24);
        gs.player.hp = 0;
        let result = gs.tick(0, 0, false);
        assert!(matches!(result.outcome, TickOutcome::GameOver));
    }

    #[test]
    fn tick_returns_level_up_when_xp_enough() {
        let mut gs = GameState::new(80, 24);
        gs.add_weapon(WeaponKind::Orbit);
        gs.xp = config::XP_THRESHOLDS[0];
        let result = gs.tick(0, 0, false);
        assert!(matches!(result.outcome, TickOutcome::LevelUp(_)));
        assert_eq!(gs.level, 2);
    }

    #[test]
    fn save_data_roundtrip() {
        let mut gs = GameState::new(80, 24);
        gs.add_weapon(WeaponKind::Laser);
        gs.elapsed_ticks = 500;
        gs.kill_count = 42;
        gs.xp = 30;
        gs.level = 3;

        let save = gs.to_save_data(vec![]);
        let restored = GameState::from_save_data(save);

        assert_eq!(restored.elapsed_ticks, 500);
        assert_eq!(restored.kill_count, 42);
        assert_eq!(restored.xp, 30);
        assert_eq!(restored.level, 3);
        assert_eq!(restored.weapons.len(), 1);
        assert_eq!(restored.player.x, gs.player.x);
    }
}
