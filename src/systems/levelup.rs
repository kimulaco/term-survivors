use crate::config;
use crate::entities::player::Player;
use crate::entities::weapon::{Weapon, WeaponKind};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Upgrade {
    NewWeapon(WeaponKind),
    LevelUpWeapon(usize), // index into weapons vec
    HealHp,
    MaxHpUp,
}

impl Upgrade {
    pub fn name(&self, weapons: &[Weapon]) -> String {
        match self {
            Upgrade::NewWeapon(kind) => format!("New: {}", kind.name()),
            Upgrade::LevelUpWeapon(idx) => {
                if let Some(w) = weapons.get(*idx) {
                    format!("{} Lv{}->{}", w.kind.name(), w.level, w.level + 1)
                } else {
                    "Level Up Weapon".to_string()
                }
            }
            Upgrade::HealHp => "Heal +30 HP".to_string(),
            Upgrade::MaxHpUp => "Max HP +20".to_string(),
        }
    }

    pub fn description(&self, weapons: &[Weapon]) -> String {
        match self {
            Upgrade::NewWeapon(kind) => kind.description().to_string(),
            Upgrade::LevelUpWeapon(idx) => {
                if let Some(w) = weapons.get(*idx) {
                    format!("More damage, faster cooldown (Lv{})", w.level + 1)
                } else {
                    String::new()
                }
            }
            Upgrade::HealHp => "Restore 30 hit points".to_string(),
            Upgrade::MaxHpUp => "Increase maximum HP by 20".to_string(),
        }
    }
}

pub fn generate_choices(weapons: &[Weapon]) -> Vec<Upgrade> {
    let mut pool: Vec<Upgrade> = Vec::new();

    // Add weapon level ups for existing weapons below max level
    for (idx, w) in weapons.iter().enumerate() {
        if w.level < 5 {
            pool.push(Upgrade::LevelUpWeapon(idx));
        }
    }

    // Add new weapons if below max
    if weapons.len() < config::MAX_WEAPONS {
        let existing: Vec<WeaponKind> = weapons.iter().map(|w| w.kind).collect();
        for kind in &[
            WeaponKind::Orbit,
            WeaponKind::Laser,
            WeaponKind::Pulse,
            WeaponKind::Drone,
        ] {
            if !existing.contains(kind) {
                pool.push(Upgrade::NewWeapon(*kind));
            }
        }
    }

    // Always offer utility upgrades
    pool.push(Upgrade::HealHp);
    pool.push(Upgrade::MaxHpUp);
    let mut rng = rand::thread_rng();
    pool.shuffle(&mut rng);
    pool.truncate(3);
    pool
}

pub fn apply_upgrade(upgrade: Upgrade, weapons: &mut Vec<Weapon>, player: &mut Player) {
    match upgrade {
        Upgrade::NewWeapon(kind) => {
            if weapons.len() < config::MAX_WEAPONS {
                weapons.push(Weapon::new(kind));
            }
        }
        Upgrade::LevelUpWeapon(idx) => {
            if let Some(w) = weapons.get_mut(idx) {
                w.level_up();
            }
        }
        Upgrade::HealHp => {
            player.heal(30);
        }
        Upgrade::MaxHpUp => {
            player.max_hp += 20;
            player.hp += 20;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_weapons() -> Vec<Weapon> {
        vec![Weapon::new(WeaponKind::Orbit)]
    }

    #[test]
    fn upgrade_name_new_weapon() {
        let weapons = make_weapons();
        let u = Upgrade::NewWeapon(WeaponKind::Laser);
        assert!(u.name(&weapons).contains("Laser"));
    }

    #[test]
    fn upgrade_name_level_up_weapon() {
        let weapons = make_weapons();
        let u = Upgrade::LevelUpWeapon(0);
        let name = u.name(&weapons);
        assert!(name.contains("Orbit"));
        assert!(name.contains("1->2"));
    }

    #[test]
    fn upgrade_name_invalid_index() {
        let weapons = make_weapons();
        let u = Upgrade::LevelUpWeapon(99);
        assert_eq!(u.name(&weapons), "Level Up Weapon");
    }

    #[test]
    fn upgrade_description_not_empty() {
        let weapons = make_weapons();
        let upgrades = [
            Upgrade::NewWeapon(WeaponKind::Pulse),
            Upgrade::LevelUpWeapon(0),
            Upgrade::HealHp,
            Upgrade::MaxHpUp,
        ];
        for u in &upgrades {
            assert!(!u.description(&weapons).is_empty());
        }
    }

    #[test]
    fn generate_choices_returns_at_most_three() {
        let weapons = make_weapons();
        let choices = generate_choices(&weapons);
        assert!(choices.len() <= 3);
        assert!(!choices.is_empty());
    }

    #[test]
    fn generate_choices_no_new_weapon_when_full() {
        let weapons = vec![
            Weapon::new(WeaponKind::Orbit),
            Weapon::new(WeaponKind::Laser),
            Weapon::new(WeaponKind::Pulse),
        ];
        for _ in 0..20 {
            let choices = generate_choices(&weapons);
            for c in &choices {
                if let Upgrade::NewWeapon(_) = c {
                    panic!("should not offer new weapon when at max");
                }
            }
        }
    }

    #[test]
    fn apply_upgrade_new_weapon() {
        let mut weapons: Vec<Weapon> = Vec::new();
        let mut player = Player::new(0, 0);
        apply_upgrade(
            Upgrade::NewWeapon(WeaponKind::Drone),
            &mut weapons,
            &mut player,
        );
        assert_eq!(weapons.len(), 1);
        assert_eq!(weapons[0].kind, WeaponKind::Drone);
    }

    #[test]
    fn apply_upgrade_level_up_weapon() {
        let mut weapons = vec![Weapon::new(WeaponKind::Orbit)];
        let mut player = Player::new(0, 0);
        apply_upgrade(Upgrade::LevelUpWeapon(0), &mut weapons, &mut player);
        assert_eq!(weapons[0].level, 2);
    }

    #[test]
    fn apply_upgrade_heal_hp() {
        let mut weapons = Vec::new();
        let mut player = Player::new(0, 0);
        player.hp = 50;
        apply_upgrade(Upgrade::HealHp, &mut weapons, &mut player);
        assert_eq!(player.hp, 80);
    }

    #[test]
    fn apply_upgrade_max_hp_up() {
        let mut weapons = Vec::new();
        let mut player = Player::new(0, 0);
        let old_max = player.max_hp;
        apply_upgrade(Upgrade::MaxHpUp, &mut weapons, &mut player);
        assert_eq!(player.max_hp, old_max + 20);
        assert_eq!(player.hp, config::PLAYER_MAX_HP + 20);
    }
}
