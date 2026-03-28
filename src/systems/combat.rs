use crate::entities::enemy::Enemy;
use crate::entities::player::Player;
use crate::entities::projectile::Projectile;

pub struct CombatResult {
    pub xp_gained: u32,
    pub kills: u32,
}

fn projectile_overlaps_enemy(proj: &Projectile, enemy: &Enemy) -> bool {
    let w = proj.width.max(1);
    let h = proj.height.max(1);
    for dy in 0..h {
        for dx in 0..w {
            if enemy.occupies(proj.x + dx, proj.y + dy) {
                return true;
            }
        }
    }
    false
}

/// Check projectile-enemy collisions, apply damage, return XP and kills
pub fn process_combat(
    projectiles: &mut [Projectile],
    enemies: &mut Vec<Enemy>,
    player_x: i32,
    player_y: i32,
) -> CombatResult {
    let mut xp_gained = 0u32;
    let mut kills = 0u32;

    for proj in projectiles.iter_mut() {
        if proj.is_expired() || proj.pierce <= 0 {
            continue;
        }
        for enemy in enemies.iter_mut() {
            if enemy.is_dead() {
                continue;
            }
            if projectile_overlaps_enemy(proj, enemy) {
                let hit = enemy.take_damage(proj.damage, proj.weapon_kind_idx as usize);
                if hit {
                    proj.pierce -= 1;
                    if proj.knockback > 0 && !enemy.is_dead() {
                        enemy.apply_knockback(player_x, player_y, proj.knockback);
                    }
                    if enemy.is_dead() {
                        xp_gained += enemy.xp_value;
                        kills += 1;
                    }
                    if proj.pierce <= 0 {
                        break;
                    }
                }
            }
        }
    }

    enemies.retain(|e| !e.is_dead());

    CombatResult { xp_gained, kills }
}

/// Check enemy-player collisions, apply damage.
/// Returns the shake power of the enemy that landed a hit, or 0 if no damage was taken.
pub fn process_enemy_contact(enemies: &[Enemy], player: &mut Player, sound_enabled: bool) -> u32 {
    for enemy in enemies {
        if enemy.collides_with_player(player.x, player.y) {
            if player.take_damage(enemy.damage, sound_enabled) {
                return enemy.shake_power;
            }
            break; // Only take damage from one enemy per tick
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::enemy::EnemyKind;
    use crate::entities::projectile::Movement;

    fn make_projectile(x: i32, y: i32, damage: i32, pierce: i32) -> Projectile {
        Projectile::new(x, y, '*', damage, 60, Movement::Static, pierce)
    }

    #[test]
    fn process_combat_hit_and_kill() {
        let mut projectiles = vec![make_projectile(5, 5, 100, 1)];
        let mut enemies = vec![Enemy::new(EnemyKind::Bug, 5, 5)];

        let result = process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert!(result.kills > 0);
        assert!(result.xp_gained > 0);
        assert!(enemies.is_empty(), "dead enemies should be removed");
    }

    #[test]
    fn process_combat_pierce_consumed() {
        let mut projectiles = vec![make_projectile(5, 5, 1, 1)];
        let mut enemies = vec![
            Enemy::new(EnemyKind::Bug, 5, 5),
            Enemy::new(EnemyKind::Bug, 5, 5),
        ];

        process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert_eq!(projectiles[0].pierce, 0);
    }

    #[test]
    fn process_combat_expired_projectile_skipped() {
        let mut projectiles = vec![Projectile::new(5, 5, '*', 100, 0, Movement::Static, 1)];
        let mut enemies = vec![Enemy::new(EnemyKind::Bug, 5, 5)];
        let hp_before = enemies[0].hp;

        process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert_eq!(enemies[0].hp, hp_before);
    }

    #[test]
    fn process_combat_zero_pierce_skipped() {
        let mut projectiles = vec![make_projectile(5, 5, 100, 0)];
        let mut enemies = vec![Enemy::new(EnemyKind::Bug, 5, 5)];
        let hp_before = enemies[0].hp;

        process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert_eq!(enemies[0].hp, hp_before);
    }

    #[test]
    fn process_combat_miss() {
        let mut projectiles = vec![make_projectile(0, 0, 100, 1)];
        let mut enemies = vec![Enemy::new(EnemyKind::Bug, 10, 10)];

        let result = process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert_eq!(result.kills, 0);
        assert_eq!(result.xp_gained, 0);
        assert_eq!(enemies.len(), 1);
    }

    #[test]
    fn process_combat_multicell_projectile_hits_offset_cell() {
        let mut projectiles = vec![make_projectile(4, 4, 100, 1).with_size(2, 2)];
        let mut enemies = vec![Enemy::new(EnemyKind::Bug, 5, 5)];

        let result = process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert!(result.kills > 0);
        assert!(enemies.is_empty());
    }

    #[test]
    fn process_combat_knockback_applied() {
        let mut projectiles = vec![make_projectile(5, 5, 1, 1).with_knockback(3)];
        let mut enemies = vec![Enemy::new(EnemyKind::Elite, 5, 5)];

        process_combat(&mut projectiles, &mut enemies, 0, 0);
        assert!(!enemies.is_empty(), "enemy should survive");
        assert_eq!(enemies[0].knockback_ticks, 3);
        assert!(enemies[0].knockback_dx != 0 || enemies[0].knockback_dy != 0);
    }

    #[test]
    fn process_enemy_contact_damages_player() {
        let enemies = vec![Enemy::new(EnemyKind::Bug, 5, 5)];
        let mut player = Player::new(5, 5);
        let hp_before = player.hp;

        process_enemy_contact(&enemies, &mut player, false);
        assert!(player.hp < hp_before);
    }

    #[test]
    fn process_enemy_contact_no_damage_when_invincible() {
        let enemies = vec![Enemy::new(EnemyKind::Bug, 5, 5)];
        let mut player = Player::new(5, 5);
        player.invincible_ticks = 10;
        let hp_before = player.hp;

        process_enemy_contact(&enemies, &mut player, false);
        assert_eq!(player.hp, hp_before);
    }

    #[test]
    fn process_enemy_contact_no_damage_when_far() {
        let enemies = vec![Enemy::new(EnemyKind::Bug, 50, 50)];
        let mut player = Player::new(0, 0);
        let hp_before = player.hp;

        process_enemy_contact(&enemies, &mut player, false);
        assert_eq!(player.hp, hp_before);
    }
}
