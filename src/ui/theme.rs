use ratatui::style::{Color, Modifier, Style};

use crate::entities::enemy::EnemyKind;

pub const DARK_BG: Color = Color::Rgb(8, 29, 53);

// Player
pub const PLAYER_COLOR: Color = Color::Cyan;
pub const PLAYER_INVINCIBLE_COLOR: Color = Color::Yellow;

// HP / XP gauge fill colors
pub const HP_HIGH_COLOR: Color = Color::Green;
pub const HP_MID_COLOR: Color = Color::Yellow;
pub const HP_LOW_COLOR: Color = Color::Red;

// Projectile — weapon-specific colors
pub const LASER_COLOR: Color = Color::Yellow;
pub const DEFAULT_PROJECTILE_COLOR: Color = Color::Cyan;

// Thunder warning indicator phases (by remaining ttl)
pub const THUNDER_ACTIVE_COLOR: Color = Color::White;
pub const THUNDER_WARN_FAR_COLOR: Color = Color::Yellow;
pub const THUNDER_WARN_MID_COLOR: Color = Color::LightYellow;
pub const THUNDER_WARN_NEAR_COLOR: Color = Color::LightRed;

// Delayed-cell preview shared by Thunder and Bomb (by remaining delay_ticks)
pub const DELAY_PREVIEW_FAR_COLOR: Color = Color::DarkGray;
pub const DELAY_PREVIEW_MID_COLOR: Color = Color::Gray;
pub const DELAY_PREVIEW_NEAR_COLOR: Color = Color::Yellow;

// Bomb fuse indicator phases (by remaining ttl)
pub const BOMB_FUSE_FAR_COLOR: Color = Color::LightYellow;
pub const BOMB_FUSE_MID_COLOR: Color = Color::Yellow;
pub const BOMB_FUSE_NEAR_COLOR: Color = Color::LightRed;
pub const BOMB_EXPLODE_COLOR: Color = Color::Yellow;

// Boss HP bar
pub const BOSS_HP_BAR_COLOR: Color = Color::Red;

/// Distinct colors per enemy kind — all in red/orange/purple/pink/brown range.
pub fn enemy_color(kind: EnemyKind) -> Color {
    match kind {
        EnemyKind::Bug => Color::LightRed,
        EnemyKind::Virus => Color::Rgb(160, 80, 220), // purple
        EnemyKind::Crash => Color::Rgb(255, 140, 0),  // orange
        EnemyKind::MemLeak => Color::Rgb(160, 100, 50), // brown
        EnemyKind::Elite => Color::Rgb(255, 160, 180), // light pink
        EnemyKind::MidBoss => Color::Rgb(180, 60, 100), // dark rose
        EnemyKind::Boss => Color::Rgb(220, 20, 60),   // crimson
    }
}

/// Label color for gauge widgets based on fill ratio.
/// Bright fill → dark text; dim fill → light text.
pub fn gauge_label_color(ratio: f64) -> Color {
    if ratio > 0.5 {
        Color::Black
    } else {
        Color::White
    }
}

/// Body text color for overlays and result popups in dark mode.
pub fn text_color(dark: bool) -> Color {
    if dark {
        Color::White
    } else {
        Color::Reset
    }
}

pub fn bg(dark: bool) -> Style {
    if dark {
        Style::default().bg(DARK_BG)
    } else {
        Style::default()
    }
}

pub fn border_style(dark: bool) -> Style {
    if dark {
        Style::default().bg(DARK_BG).fg(Color::White)
    } else {
        Style::default()
    }
}

pub fn gauge_fg_style(color: Color, dark: bool) -> Style {
    if dark {
        Style::default().fg(color).bg(DARK_BG)
    } else {
        Style::default().fg(color)
    }
}

pub fn popup_header_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}
