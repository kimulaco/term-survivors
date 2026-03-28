#[cfg(all(not(test), target_os = "macos"))]
pub fn play_level_up(sound_enabled: bool) {
    if !sound_enabled {
        return;
    }
    let path = std::env::var("TERM_SURVIVORS_LEVELUP_SOUND")
        .unwrap_or_else(|_| "/System/Library/Sounds/Glass.aiff".to_string());
    let _ = std::process::Command::new("afplay").arg(path).spawn();
}

#[cfg(all(not(test), not(target_os = "macos")))]
pub fn play_level_up(sound_enabled: bool) {
    if !sound_enabled {
        return;
    }
    use std::io::{self, Write};
    let _ = io::stderr().write_all(b"\x07");
}

#[cfg(test)]
pub fn play_level_up(_sound_enabled: bool) {}

#[cfg(all(not(test), target_os = "macos"))]
pub fn play_player_hurt(sound_enabled: bool) {
    if !sound_enabled {
        return;
    }
    let path = std::env::var("TERM_SURVIVORS_HURT_SOUND")
        .unwrap_or_else(|_| "/System/Library/Sounds/Basso.aiff".to_string());
    let _ = std::process::Command::new("afplay").arg(path).spawn();
}

#[cfg(all(not(test), not(target_os = "macos")))]
pub fn play_player_hurt(sound_enabled: bool) {
    if !sound_enabled {
        return;
    }
    use std::io::{self, Write};
    let _ = io::stderr().write_all(b"\x07");
}

#[cfg(test)]
pub fn play_player_hurt(_sound_enabled: bool) {}
