//! Human-readable formatting for sizes and ages.
//!
//! Size/age unit logic is adapted from kondo (MIT, © 2020 Trent Billington);
//! see ATTRIBUTION.md.

use std::time::{SystemTime, UNIX_EPOCH};

const KIB: u64 = 1 << 10;
const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;
const TIB: u64 = 1 << 40;

/// Format a byte count using binary (KiB/MiB/…) units, e.g. `412.0 MiB`.
pub fn size(bytes: u64) -> String {
    let (value, unit) = match bytes {
        b if b < KIB => return format!("{b} B"),
        b if b < MIB => (b as f64 / KIB as f64, "KiB"),
        b if b < GIB => (b as f64 / MIB as f64, "MiB"),
        b if b < TIB => (b as f64 / GIB as f64, "GiB"),
        b => (b as f64 / TIB as f64, "TiB"),
    };
    format!("{value:.1} {unit}")
}

/// Format the age of `t` relative to now as a coarse `5h` / `2d` / `31d` string.
/// Returns `?` if the timestamp is missing or in the future.
pub fn age(t: Option<SystemTime>) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = MINUTE * 60;
    const DAY: u64 = HOUR * 24;

    let Some(t) = t else { return "?".to_string() };
    let (Ok(then), Ok(now)) = (
        t.duration_since(UNIX_EPOCH),
        SystemTime::now().duration_since(UNIX_EPOCH),
    ) else {
        return "?".to_string();
    };
    let Some(secs) = now.as_secs().checked_sub(then.as_secs()) else {
        return "now".to_string();
    };
    match secs {
        s if s < MINUTE => "now".to_string(),
        s if s < HOUR => format!("{}m", s / MINUTE),
        s if s < DAY => format!("{}h", s / HOUR),
        s => format!("{}d", s / DAY),
    }
}
