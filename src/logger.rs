use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static WRITER: OnceLock<Mutex<BufWriter<File>>> = OnceLock::new();

fn log_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(crate::save::SAVE_DIR).join("logs"))
}

pub fn init() {
    let Some(dir) = log_dir() else { return };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(file) = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dir.join("latest.log"))
    else {
        return;
    };
    let _ = WRITER.set(Mutex::new(BufWriter::new(file)));
}

fn now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time = (secs % 86400) as u32;
    let (year, month, day) = epoch_days_to_ymd(secs / 86400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        time / 3600,
        (time % 3600) / 60,
        time % 60,
    )
}

/// Algorithm: Howard Hinnant's civil_from_days.
fn epoch_days_to_ymd(days: u64) -> (u32, u32, u32) {
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y as u32, mo, d)
}

fn write_log(level: &str, msg: &str) {
    let Some(mutex) = WRITER.get() else { return };
    let Ok(mut w) = mutex.lock() else { return };
    let _ = writeln!(w, "[{} {}] {}", now_utc(), level, msg);
    let _ = w.flush();
}

pub fn info(msg: &str) {
    write_log("INFO", msg);
}

pub fn error(msg: &str) {
    write_log("ERROR", msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_days_to_ymd_unix_epoch() {
        assert_eq!(epoch_days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn epoch_days_to_ymd_known_date() {
        // 2026-04-06 is day 20549 since Unix epoch
        assert_eq!(epoch_days_to_ymd(20549), (2026, 4, 6));
    }

    #[test]
    fn epoch_days_to_ymd_leap_day() {
        // 2000-02-29 is a leap day
        // 2000-02-29: days since epoch = 11016
        assert_eq!(epoch_days_to_ymd(11016), (2000, 2, 29));
    }
}
