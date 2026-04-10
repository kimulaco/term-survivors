use crate::config;
use crate::entities::weapon::WeaponKind;
use crate::save::{GameSaveData, Settings};
use crate::systems::levelup::Upgrade;
use crate::systems::session::{GameState, TickOutcome};
use rand::seq::SliceRandom;

const ALL_WEAPONS: [WeaponKind; 6] = [
    WeaponKind::Orbit,
    WeaponKind::Laser,
    WeaponKind::Drone,
    WeaponKind::Bomb,
    WeaponKind::Scatter,
    WeaponKind::Thunder,
];

pub enum AppPhase {
    Title,
    WeaponSelect(Vec<WeaponKind>, usize),
    Playing,
    Paused,
    LevelUp(Vec<Upgrade>, usize),
    GameOver,
    Cleared,
}

pub struct App {
    pub phase: AppPhase,
    pub game: GameState,
    pub save: Settings,
    pub has_session: bool,
    pub dx: i32,
    pub dy: i32,
    pub screen_shake_ticks: u32,
    pub damage_flash_ticks: u32,
}

impl App {
    pub fn new(field_width: i32, field_height: i32) -> Self {
        let has_session = GameSaveData::exists();
        let mut app = Self {
            phase: AppPhase::Title,
            game: GameState::new(field_width, field_height),
            save: Settings::load(),
            has_session,
            dx: 0,
            dy: 0,
            screen_shake_ticks: 0,
            damage_flash_ticks: 0,
        };
        if has_session && app.save.auto_restart {
            app.resume_game();
        }
        app
    }

    pub fn screen_shake_offset(&self) -> (i32, i32) {
        if self.screen_shake_ticks == 0 {
            return (0, 0);
        }
        let mag = if self.screen_shake_ticks > config::SCREEN_SHAKE_MAGNITUDE_THRESHOLD {
            2
        } else {
            1
        };
        match self.screen_shake_ticks % config::SCREEN_SHAKE_PATTERN_CYCLE {
            0 => (mag, 0),
            1 => (-mag, 0),
            2 => (0, 1),
            _ => (0, -1),
        }
    }

    pub fn is_damage_flash_active(&self) -> bool {
        self.damage_flash_ticks > 0
            && self.damage_flash_ticks % config::DAMAGE_FLASH_CYCLE
                < config::DAMAGE_FLASH_ON_THRESHOLD
    }

    pub fn start_game(&mut self) {
        GameSaveData::delete();
        self.has_session = false;
        self.game = GameState::new(self.game.field_width, self.game.field_height);
        let mut pool: Vec<WeaponKind> = ALL_WEAPONS.to_vec();
        pool.shuffle(&mut rand::thread_rng());
        pool.truncate(3);
        self.phase = AppPhase::WeaponSelect(pool, 0);
    }

    pub fn select_starting_weapon(&mut self, index: usize) {
        if let AppPhase::WeaponSelect(choices, _) = &self.phase {
            if let Some(&kind) = choices.get(index) {
                self.game.add_weapon(kind);
                crate::logger::info(&format!("game started - weapon: {}", kind.name()));
                self.phase = AppPhase::Playing;
            }
        }
    }

    pub fn resume_game(&mut self) {
        if let Some(data) = GameSaveData::load() {
            let fw = self.game.field_width;
            let fh = self.game.field_height;
            let pending = data.pending_upgrades.clone();
            self.game = GameState::from_save_data(data);
            self.game.resize(fw, fh);
            GameSaveData::delete();
            self.has_session = false;
            crate::logger::info(&format!(
                "session resumed - level {}, time {}",
                self.game.level,
                Settings::format_ticks(self.game.elapsed_ticks),
            ));
            if pending.is_empty() {
                self.phase = AppPhase::Playing;
            } else {
                self.phase = AppPhase::LevelUp(pending, 0);
            }
        }
    }

    pub fn save_session(&self) {
        let pending = if let AppPhase::LevelUp(choices, _) = &self.phase {
            choices.clone()
        } else {
            vec![]
        };
        if let AppPhase::Playing | AppPhase::Paused | AppPhase::LevelUp(_, _) = &self.phase {
            self.game.to_save_data(pending).save();
        }
    }

    pub fn toggle_auto_restart(&mut self) {
        self.save.toggle_auto_restart();
    }

    pub fn toggle_dark_mode(&mut self) {
        self.save.toggle_dark_mode();
    }

    pub fn pause(&mut self) {
        if matches!(self.phase, AppPhase::Playing) {
            self.phase = AppPhase::Paused;
        }
    }

    pub fn resume_from_pause(&mut self) {
        if matches!(self.phase, AppPhase::Paused) {
            self.phase = AppPhase::Playing;
        }
    }

    pub fn tick(&mut self) {
        if self.screen_shake_ticks > 0 {
            self.screen_shake_ticks -= 1;
        }
        if self.damage_flash_ticks > 0 {
            self.damage_flash_ticks -= 1;
        }
        if let AppPhase::Playing = &self.phase {
            let result = self.game.tick(self.dx, self.dy);
            if result.screen_shake > 0 {
                self.screen_shake_ticks = result.screen_shake;
                self.damage_flash_ticks = config::DAMAGE_FLASH_DURATION;
            }
            match result.outcome {
                TickOutcome::Continue => {}
                TickOutcome::LevelUp(choices) => {
                    self.phase = AppPhase::LevelUp(choices, 0);
                }
                TickOutcome::GameOver => {
                    crate::logger::info(&format!(
                        "game over - level {}, time {}",
                        self.game.level,
                        Settings::format_ticks(self.game.elapsed_ticks),
                    ));
                    GameSaveData::delete();
                    if self.save.auto_restart {
                        self.start_game();
                    } else {
                        self.phase = AppPhase::GameOver;
                    }
                }
                TickOutcome::Cleared => {
                    crate::logger::info(&format!(
                        "game cleared - level {}, time {}",
                        self.game.level,
                        Settings::format_ticks(self.game.elapsed_ticks),
                    ));
                    GameSaveData::delete();
                    if self.save.auto_restart {
                        self.start_game();
                    } else {
                        self.phase = AppPhase::Cleared;
                    }
                }
            }
        }
    }

    pub fn select_upgrade(&mut self, index: usize) {
        if let AppPhase::LevelUp(choices, _) = &self.phase {
            if let Some(&upgrade) = choices.get(index) {
                crate::logger::info(&format!(
                    "level {} upgrade: {}",
                    self.game.level,
                    upgrade.name(&self.game.weapons),
                ));
                self.game.apply_upgrade(upgrade);
                self.phase = AppPhase::Playing;
            }
        }
    }

    pub fn return_to_title(&mut self) {
        let pending = if let AppPhase::LevelUp(choices, _) = &self.phase {
            choices.clone()
        } else {
            vec![]
        };
        self.game.to_save_data(pending).save();
        self.has_session = true;
        self.phase = AppPhase::Title;
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.game.resize(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_only_from_playing() {
        let mut app = App::new(40, 24);
        app.phase = AppPhase::Title;
        app.pause();
        assert!(matches!(app.phase, AppPhase::Title));
        app.phase = AppPhase::Playing;
        app.pause();
        assert!(matches!(app.phase, AppPhase::Paused));
    }

    #[test]
    fn resume_from_pause_returns_to_playing() {
        let mut app = App::new(40, 24);
        app.phase = AppPhase::Paused;
        app.resume_from_pause();
        assert!(matches!(app.phase, AppPhase::Playing));
    }
}
