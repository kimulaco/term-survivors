#[cfg(all(not(test), target_os = "macos"))]
pub fn play_level_up(muted: bool) {
    if muted {
        return;
    }
    let path = std::env::var("TERM_SURVIVORS_LEVELUP_SOUND")
        .unwrap_or_else(|_| "/System/Library/Sounds/Glass.aiff".to_string());
    let _ = std::process::Command::new("afplay").arg(path).spawn();
}

#[cfg(all(not(test), not(target_os = "macos")))]
pub fn play_level_up(muted: bool) {
    if muted {
        return;
    }
    use std::io::{self, Write};
    let _ = io::stderr().write_all(b"\x07");
}

#[cfg(test)]
pub fn play_level_up(_muted: bool) {}

#[cfg(all(not(test), target_os = "macos"))]
pub fn play_player_hurt(muted: bool) {
    if muted {
        return;
    }
    let path = std::env::var("TERM_SURVIVORS_HURT_SOUND")
        .unwrap_or_else(|_| "/System/Library/Sounds/Basso.aiff".to_string());
    let _ = std::process::Command::new("afplay").arg(path).spawn();
}

#[cfg(all(not(test), not(target_os = "macos")))]
pub fn play_player_hurt(muted: bool) {
    if muted {
        return;
    }
    use std::io::{self, Write};
    let _ = io::stderr().write_all(b"\x07");
}

#[cfg(test)]
pub fn play_player_hurt(_muted: bool) {}
