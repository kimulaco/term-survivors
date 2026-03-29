use crate::audio;
use crate::config;
use serde::{Deserialize, Serialize};

/// The 4 cardinal directions the player can face.
/// Diagonal inputs snap to the dominant axis (ties prefer horizontal).
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FacingDir {
    Right,
    Left,
    Down,
    Up,
}

impl FacingDir {
    pub fn from_input(dx: i32, dy: i32) -> Option<Self> {
        if dx == 0 && dy == 0 {
            return None;
        }
        if dx.abs() >= dy.abs() {
            match dx.signum() {
                1 => Some(FacingDir::Right),
                -1 => Some(FacingDir::Left),
                _ => None,
            }
        } else {
            match dy.signum() {
                1 => Some(FacingDir::Down),
                -1 => Some(FacingDir::Up),
                _ => None,
            }
        }
    }

    /// Returns the (dx, dy) unit vector for this direction.
    pub fn to_dir(self) -> (i32, i32) {
        match self {
            FacingDir::Right => (1, 0),
            FacingDir::Left => (-1, 0),
            FacingDir::Down => (0, 1),
            FacingDir::Up => (0, -1),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Player {
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub(crate) invincible_ticks: u32,
    pub facing: FacingDir,
}

impl Player {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            hp: config::PLAYER_MAX_HP,
            max_hp: config::PLAYER_MAX_HP,
            invincible_ticks: 0,
            facing: FacingDir::Right,
        }
    }

    pub fn update(&mut self, dx: i32, dy: i32, width: i32, height: i32) {
        if self.invincible_ticks > 0 {
            self.invincible_ticks -= 1;
        }

        if let Some(dir) = FacingDir::from_input(dx, dy) {
            self.facing = dir;
        }

        if dx != 0 {
            self.x = (self.x + dx).clamp(0, width - 1);
        }
        if dy != 0 {
            self.y = (self.y + dy).clamp(0, height - 1);
        }
    }

    /// Apply damage to the player. Returns true if damage landed, false if blocked by invincibility.
    pub fn take_damage(&mut self, damage: i32, sound_enabled: bool) -> bool {
        if self.invincible_ticks > 0 {
            return false;
        }
        self.hp = (self.hp - damage).max(0);
        self.invincible_ticks = config::PLAYER_INVINCIBLE_TICKS;
        audio::play_player_hurt(sound_enabled);
        true
    }

    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_initializes_correctly() {
        let p = Player::new(10, 20);
        assert_eq!(p.x, 10);
        assert_eq!(p.y, 20);
        assert_eq!(p.hp, config::PLAYER_MAX_HP);
        assert_eq!(p.max_hp, config::PLAYER_MAX_HP);
        assert_eq!(p.invincible_ticks, 0);
    }

    #[test]
    fn update_moves_player() {
        let mut p = Player::new(5, 5);
        p.update(1, 0, 20, 20);
        assert_eq!(p.x, 6);
        assert_eq!(p.y, 5);

        p.update(0, -1, 20, 20);
        assert_eq!(p.x, 6);
        assert_eq!(p.y, 4);
    }

    #[test]
    fn update_no_movement_when_zero() {
        let mut p = Player::new(5, 5);
        p.update(0, 0, 20, 20);
        assert_eq!(p.x, 5);
        assert_eq!(p.y, 5);
    }

    #[test]
    fn update_clamps_to_bounds() {
        let mut p = Player::new(0, 0);
        p.update(-1, -1, 10, 10);
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);

        let mut p = Player::new(9, 9);
        p.update(1, 1, 10, 10);
        assert_eq!(p.x, 9);
        assert_eq!(p.y, 9);
    }

    #[test]
    fn update_decrements_invincible_ticks() {
        let mut p = Player::new(5, 5);
        p.invincible_ticks = 3;
        p.update(0, 0, 20, 20);
        assert_eq!(p.invincible_ticks, 2);
    }

    #[test]
    fn take_damage_reduces_hp() {
        let mut p = Player::new(0, 0);
        p.take_damage(30, false);
        assert_eq!(p.hp, config::PLAYER_MAX_HP - 30);
        assert_eq!(p.invincible_ticks, config::PLAYER_INVINCIBLE_TICKS);
    }

    #[test]
    fn take_damage_blocked_while_invincible() {
        let mut p = Player::new(0, 0);
        p.invincible_ticks = 10;
        let hp_before = p.hp;
        p.take_damage(50, false);
        assert_eq!(p.hp, hp_before);
    }

    #[test]
    fn take_damage_hp_floors_at_zero() {
        let mut p = Player::new(0, 0);
        p.take_damage(9999, false);
        assert_eq!(p.hp, 0);
    }

    #[test]
    fn is_dead_when_hp_zero() {
        let mut p = Player::new(0, 0);
        assert!(!p.is_dead());
        p.hp = 0;
        assert!(p.is_dead());
    }

    #[test]
    fn heal_restores_hp() {
        let mut p = Player::new(0, 0);
        p.hp = 50;
        p.heal(20);
        assert_eq!(p.hp, 70);
    }

    #[test]
    fn heal_capped_at_max_hp() {
        let mut p = Player::new(0, 0);
        p.hp = 90;
        p.heal(999);
        assert_eq!(p.hp, p.max_hp);
    }
}
