use crate::audio;
use crate::config;
use crate::entities::weapon::WeaponKind;
use crate::save::{GameSaveData, Settings};
use crate::systems::levelup::Upgrade;
use crate::systems::session::{GameState, TickOutcome};

pub const WEAPON_CHOICES: [WeaponKind; 4] = [
    WeaponKind::Orbit,
    WeaponKind::Laser,
    WeaponKind::Pulse,
    WeaponKind::Drone,
];

pub enum AppPhase {
    Title,
    WeaponSelect(usize),
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
}

impl App {
    pub fn new(field_width: i32, field_height: i32) -> Self {
        Self {
            phase: AppPhase::Title,
            game: GameState::new(field_width, field_height),
            save: Settings::load(),
            has_session: GameSaveData::exists(),
            dx: 0,
            dy: 0,
            screen_shake_ticks: 0,
        }
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

    pub fn start_game(&mut self) {
        GameSaveData::delete();
        self.has_session = false;
        self.game = GameState::new(self.game.field_width, self.game.field_height);
        self.phase = AppPhase::WeaponSelect(0);
    }

    pub fn select_starting_weapon(&mut self, index: usize) {
        if let Some(&kind) = WEAPON_CHOICES.get(index) {
            self.game.add_weapon(kind);
            self.phase = AppPhase::Playing;
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

    pub fn toggle_sound(&mut self) {
        self.save.toggle_sound();
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
        if let AppPhase::Playing = &self.phase {
            let result = self.game.tick(self.dx, self.dy, self.save.sound_enabled);
            if result.screen_shake > 0 {
                self.screen_shake_ticks = result.screen_shake;
            }
            match result.outcome {
                TickOutcome::Continue => {}
                TickOutcome::LevelUp(choices) => {
                    audio::play_level_up(self.save.sound_enabled);
                    self.phase = AppPhase::LevelUp(choices, 0);
                }
                TickOutcome::GameOver => {
                    GameSaveData::delete();
                    self.phase = AppPhase::GameOver;
                }
                TickOutcome::Cleared => {
                    GameSaveData::delete();
                    self.phase = AppPhase::Cleared;
                }
            }
        }
    }

    pub fn select_upgrade(&mut self, index: usize) {
        if let AppPhase::LevelUp(choices, _) = &self.phase {
            if let Some(&upgrade) = choices.get(index) {
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
