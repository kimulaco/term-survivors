pub const FPS: u32 = 60;
pub const TICK_DURATION_MS: u64 = 1000 / FPS as u64;

pub const MIN_WIDTH: u16 = 60;
pub const MIN_HEIGHT: u16 = 16;

pub const MAX_FIELD_WIDTH: i32 = 78;
pub const MAX_FIELD_HEIGHT: i32 = 20;

// Status bar height (top)
pub const STATUS_BAR_HEIGHT: u16 = 6;

/// Y-axis aspect ratio correction for terminal cells (roughly 2:1 height:width).
pub const TERMINAL_Y_ASPECT: f64 = 0.5;

// Screen shake effect parameters
/// shake_ticks above this threshold use magnitude 2; at or below use magnitude 1.
pub const SCREEN_SHAKE_MAGNITUDE_THRESHOLD: u32 = 8;
/// Number of ticks per shake direction cycle (right → left → down → up).
pub const SCREEN_SHAKE_PATTERN_CYCLE: u32 = 4;

// Damage flash effect parameters
/// Total ticks for the damage border flash (2 blinks, ≈ 533ms at 60 FPS).
pub const DAMAGE_FLASH_DURATION: u32 = 32;
/// Ticks per one blink cycle (on + off = 16 ticks).
pub const DAMAGE_FLASH_CYCLE: u32 = 16;
/// Ticks within a cycle below which the border is LightRed (0..8 = ON, 8..16 = OFF).
pub const DAMAGE_FLASH_ON_THRESHOLD: u32 = 8;
