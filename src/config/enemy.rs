/// Controls how an enemy kind enters the game.
#[derive(Clone, Copy)]
pub enum SpawnBehavior {
    /// Controlled by SPAWN_TABLE (regular enemies).
    FromTable,
    /// Spawns once at a fixed tick (final boss).
    Once { spawn_tick: u32 },
    /// Spawns periodically after first_tick until replaced by the boss.
    Periodic {
        first_tick: u32,
        interval_ticks: u32,
        max_alive: usize,
    },
}

/// Unified enemy stat block used for all enemy kinds.
pub struct EnemyStats {
    #[allow(dead_code)]
    pub name: &'static str,
    pub hp: i32,
    pub speed: f64,
    pub damage: i32,
    pub xp_value: u32,
    pub glyph: char,
    pub shake_power: u32,
    /// Cell width (1 for regular enemies, larger for bosses)
    pub width: i32,
    /// Cell height (1 for regular enemies, larger for bosses)
    pub height: i32,
    /// Knockback distance divisor (1 = full knockback, >1 = reduced)
    pub knockback_divisor: i32,
    pub spawn: SpawnBehavior,
}

// Enemy stats: regular enemies
pub const ENEMY_BUG: EnemyStats = EnemyStats {
    name: "Bug",
    hp: 30,
    speed: 3.0,
    damage: 10,
    xp_value: 5,
    glyph: 'z',
    shake_power: 5,
    width: 1,
    height: 1,
    knockback_divisor: 1,
    spawn: SpawnBehavior::FromTable,
};
pub const ENEMY_VIRUS: EnemyStats = EnemyStats {
    name: "Virus",
    hp: 60,
    speed: 4.0,
    damage: 20,
    xp_value: 8,
    glyph: 'V',
    shake_power: 5,
    width: 1,
    height: 1,
    knockback_divisor: 1,
    spawn: SpawnBehavior::FromTable,
};
pub const ENEMY_CRASH: EnemyStats = EnemyStats {
    name: "Crash",
    hp: 40,
    speed: 5.0,
    damage: 20,
    xp_value: 12,
    glyph: '!',
    shake_power: 5,
    width: 1,
    height: 1,
    knockback_divisor: 1,
    spawn: SpawnBehavior::FromTable,
};
pub const ENEMY_MEMLEAK: EnemyStats = EnemyStats {
    name: "MemLeak",
    hp: 250,
    speed: 2.0,
    damage: 20,
    xp_value: 15,
    glyph: 'M',
    shake_power: 5,
    width: 1,
    height: 1,
    knockback_divisor: 1,
    spawn: SpawnBehavior::FromTable,
};
pub const ENEMY_ELITE: EnemyStats = EnemyStats {
    name: "Elite",
    hp: 250,
    speed: 4.0,
    damage: 30,
    xp_value: 25,
    glyph: 'E',
    shake_power: 8,
    width: 1,
    height: 1,
    knockback_divisor: 1,
    spawn: SpawnBehavior::FromTable,
};

// Mid-boss (2x2, spawns periodically from 0:40 until Boss arrives)
pub const ENEMY_MIDBOSS: EnemyStats = EnemyStats {
    name: "Heap Corruptor",
    hp: 1000,
    speed: 2.5,
    damage: 50,
    xp_value: 150,
    glyph: 'H',
    shake_power: 12,
    width: 2,
    height: 2,
    knockback_divisor: 2,
    spawn: SpawnBehavior::Periodic {
        first_tick: 60 * 40,     // 0:40
        interval_ticks: 60 * 60, // respawn every 60s
        max_alive: 1,
    },
};
// Final boss (spawns at 4:30, defeating it clears the game)
pub const ENEMY_BOSS: EnemyStats = EnemyStats {
    name: "Kernel Panic",
    hp: 1500,
    speed: 2.0,
    damage: 60,
    xp_value: 200,
    glyph: 'B',
    shake_power: 12,
    width: 6,
    height: 3,
    knockback_divisor: 3,
    spawn: SpawnBehavior::Once {
        spawn_tick: 60 * 270, // 4:30
    },
};

// Spawn table: (tick_threshold, spawn_interval_ticks, kinds_available)
// kinds_available is a bitmask: 1=Bug, 2=Virus, 4=Crash, 8=MemLeak, 16=Elite
pub const SPAWN_TABLE: [(u32, u32, u8); 8] = [
    (0, 60, 0b00001),        // 0:00 - Bug only, every 1.0s
    (60 * 20, 50, 0b00011),  // 0:20 - +Virus, every ~0.83s
    (60 * 40, 40, 0b00011),  // 0:40 - same pool, every 0.67s
    (60 * 60, 32, 0b00111),  // 1:00 - +Crash, every 0.53s
    (60 * 90, 24, 0b01111),  // 1:30 - +MemLeak, every 0.40s
    (60 * 150, 18, 0b11111), // 2:30 - +Elite, every 0.30s
    (60 * 210, 13, 0b11111), // 3:30 - every 0.22s
    (60 * 270, 8, 0b11111),  // 4:30 - max intensity (boss arrives)
];
