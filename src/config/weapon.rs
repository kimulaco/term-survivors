/// Cooldown behaviour for a weapon kind.
#[derive(Clone, Copy)]
pub enum CooldownConfig {
    /// Fixed cooldown per level (index 0 = Lv1).
    PerLevel([u32; 5]),
    /// Base cooldown reduced 10% per level above 1, floored at 5 ticks.
    Scaling(u32),
}

/// Fire parameters specific to each weapon kind.
#[derive(Clone, Copy)]
pub enum WeaponFireConfig {
    Orbit {
        radius: i32,
        radius_per_level: i32,
        rotation_speed: f64,
        base_count: usize,
        width: i32,
        height: i32,
    },
    Laser {
        base_length: i32,
        length_per_level: i32,
        knockback: i32,
    },
    Pulse {
        radius: i32,
        angle_divisions: usize,
        knockback: i32,
    },
    Drone,
}

/// Unified weapon stat block used for all weapon kinds.
pub struct WeaponStats {
    pub name: &'static str,
    pub description: &'static str,
    pub damage_table: [i32; 5],
    pub cooldown: CooldownConfig,
    /// Ticks before the same weapon can hit the same enemy again.
    pub hit_cooldown: u32,
    pub fire: WeaponFireConfig,
}

pub const WEAPON_ORBIT: WeaponStats = WeaponStats {
    name: "Orbit",
    description: "Orbs circle around you",
    damage_table: [15, 18, 22, 30, 40],
    cooldown: CooldownConfig::Scaling(10),
    hit_cooldown: 10,
    fire: WeaponFireConfig::Orbit {
        radius: 7,
        radius_per_level: 1,
        rotation_speed: 0.08,
        base_count: 2,
        width: 2,
        height: 2,
    },
};

pub const WEAPON_LASER: WeaponStats = WeaponStats {
    name: "Laser",
    description: "Fires beams in 4 directions",
    damage_table: [20, 32, 46, 62, 80],
    cooldown: CooldownConfig::Scaling(45),
    hit_cooldown: 8,
    fire: WeaponFireConfig::Laser {
        base_length: 8,
        length_per_level: 2,
        knockback: 2,
    },
};

pub const WEAPON_PULSE: WeaponStats = WeaponStats {
    name: "Pulse",
    description: "Short-range blast around you",
    damage_table: [40, 55, 70, 85, 100],
    cooldown: CooldownConfig::PerLevel([120, 105, 90, 75, 60]),
    hit_cooldown: 6,
    fire: WeaponFireConfig::Pulse {
        radius: 2,
        angle_divisions: 24,
        knockback: 3,
    },
};

pub const WEAPON_DRONE: WeaponStats = WeaponStats {
    name: "Drone",
    description: "Homing projectiles seek enemies",
    damage_table: [12, 17, 23, 30, 38],
    cooldown: CooldownConfig::Scaling(60),
    hit_cooldown: 10,
    fire: WeaponFireConfig::Drone,
};

/// Returns the hit cooldown for a weapon by its kind index (Orbit=0, Laser=1, Pulse=2, Drone=3).
pub fn weapon_hit_cooldown(weapon_kind_idx: usize) -> u32 {
    match weapon_kind_idx {
        0 => WEAPON_ORBIT.hit_cooldown,
        1 => WEAPON_LASER.hit_cooldown,
        2 => WEAPON_PULSE.hit_cooldown,
        _ => WEAPON_DRONE.hit_cooldown,
    }
}
