use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::entities::enemy::Enemy;
use crate::entities::player::Player;
use crate::entities::weapon::Weapon;
use crate::systems::levelup::Upgrade;
use crate::systems::session::BossState;

const SAVE_DIR: &str = ".term_survivors";
const SETTINGS_FILENAME: &str = "settings.json";
const SESSION_FILENAME: &str = "session.json";

fn save_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(SAVE_DIR))
}

fn ensure_save_dir() -> Option<PathBuf> {
    let dir = save_dir()?;
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("Failed to create save directory {}: {}", dir.display(), e);
            return None;
        }
    }
    Some(dir)
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Settings {
    pub sound_enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GameSaveData {
    pub(crate) player: Player,
    pub(crate) enemies: Vec<Enemy>,
    pub(crate) weapons: Vec<Weapon>,
    pub(crate) elapsed_ticks: u32,
    pub(crate) kill_count: u32,
    pub(crate) xp: u32,
    pub(crate) level: u32,
    pub(crate) spawn_timer: u32,
    #[serde(default)]
    pub(crate) mid_boss_spawn_timer: u32,
    #[serde(default)]
    pub(crate) boss_state: BossState,
    pub(crate) field_width: i32,
    pub(crate) field_height: i32,
    #[serde(default)]
    pub(crate) pending_upgrades: Vec<Upgrade>,
}

impl Settings {
    fn path() -> Option<PathBuf> {
        save_dir().map(|d| d.join(SETTINGS_FILENAME))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if ensure_save_dir().is_none() {
            return;
        }
        if let Some(path) = Self::path() {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("Failed to save settings to {}: {}", path.display(), e);
                }
            }
        }
    }

    pub fn toggle_sound(&mut self) {
        self.sound_enabled = !self.sound_enabled;
        self.save();
    }

    pub fn format_ticks(ticks: u32) -> String {
        let total_secs = ticks / 60;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }
}

impl GameSaveData {
    fn path() -> Option<PathBuf> {
        save_dir().map(|d| d.join(SESSION_FILENAME))
    }

    pub fn load() -> Option<Self> {
        Self::path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    pub fn save(&self) {
        if ensure_save_dir().is_none() {
            return;
        }
        if let Some(path) = Self::path() {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                if let Err(e) = fs::write(&path, json) {
                    eprintln!("Failed to save session to {}: {}", path.display(), e);
                }
            }
        }
    }

    pub fn delete() {
        if let Some(path) = Self::path() {
            let _ = fs::remove_file(path);
        }
    }

    pub fn exists() -> bool {
        Self::path().map(|p| p.exists()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_ticks_zero() {
        assert_eq!(Settings::format_ticks(0), "00:00");
    }

    #[test]
    fn format_ticks_one_minute() {
        assert_eq!(Settings::format_ticks(3600), "01:00");
    }

    #[test]
    fn format_ticks_mixed() {
        assert_eq!(Settings::format_ticks(60 * 90 + 30), "01:30");
    }

    #[test]
    fn format_ticks_ten_minutes() {
        assert_eq!(Settings::format_ticks(60 * 600), "10:00");
    }

    #[test]
    fn settings_default_sound_disabled() {
        assert!(!Settings::default().sound_enabled);
    }
}
