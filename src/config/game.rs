// XP thresholds per level (cumulative)
pub const XP_THRESHOLDS: [u32; 20] = [
    10, 25, 50, 80, 120, 170, 230, 300, 380, 470, 570, 680, 800, 930, 1070, 1220, 1380, 1550, 1730,
    1920,
];

pub const MAX_WEAPONS: usize = 3;

// Game duration: 5 minutes (countdown timer; boss must be defeated to clear)
pub const GAME_DURATION_TICKS: u32 = 60 * 60 * 5; // 18000 ticks

// Maximum number of enemies on screen at once (farthest are culled)
pub const MAX_ENEMY_COUNT: usize = 200;
