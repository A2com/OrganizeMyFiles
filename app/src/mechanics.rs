//! Constants, formulas and rules recovered from the original program's code.
//! Source addresses for every value are documented in `MECHANICS.md` and
//! `mechanics.json`. Do not change these without changing those files too.

// ---- Content extraction caps (observed) -----------------------------------
/// Text / PDF / office document extraction cap, 100 MB (0x1001e30f0).
pub const TEXT_LIMIT_BYTES: u64 = 104857600;
/// Image & scanned-PDF OCR cap, 25 MB, Pro only (0x1001e3530).
pub const OCR_LIMIT_BYTES: u64 = 26214400;
/// OCR image dimension cap in pixels: width > 12000 or height >= 12001 refused
/// (0x2ee0/0x2ee1 in ContentExtractor.extractImageOCR at 0x100150c7c).
pub const OCR_IMAGE_DIMENSION_LIMIT: u32 = 12000;
/// App-settings key for the history retention, read by AppModel (0x10000a048)
/// and written by setHistoryMaxDays (0x10000c210), string at 0x1001ded10.
pub const HISTORY_RETENTION_SETTING_KEY: &str = "history_max_days";
/// Action parameter key for the conflict policy, ActionParam.onConflict
/// (0x100174b34), string at 0x1001ab120; values "replace" or "skip".
pub const ON_CONFLICT_PARAM_KEY: &str = "on_conflict";

// ---- Tunables (estimated: properties seen, numbers not recovered) ---------
pub const PREVIEW_MATCH_LIMIT: usize = 200;
pub const HISTORY_PAGE_SIZE: usize = 50;
pub const SCRIPT_TIMEOUT_SECONDS: u64 = 60;
pub const HISTORY_RETENTION_DEFAULT_DAYS: u32 = 30;

// ---- Finder color labels (observed, FinderTags.colorIndex 0x1001539b8) ----
/// Case-insensitive color name -> Finder label index (0 = none):
/// gray=1, green=2, purple=3, blue=4, yellow=5, red=6, orange=7.
/// The label is appended as "<name>\n<index>" to the user-tags value.
pub const FINDER_COLOR_INDEX: &[(&str, u8)] =
    &[("gray", 1), ("green", 2), ("purple", 3), ("blue", 4), ("yellow", 5), ("red", 6), ("orange", 7)];

// ---- History purge formula (observed, Database.purgeHistory 0x10017a438) ---
/// Purge cutoff seconds: days * 86400, subtracted from "now" (timezone
/// identifier built from the two bytes 0x54, 0x55 = "UTC"; the bound is
/// formatted as an ISO date and compared with datetime(created_at)).
pub const HISTORY_PURGE_SECONDS_PER_DAY: u64 = 86400;

// ---- Filename rules (observed, 0x100138c54 family) ------------------------
pub const FILENAME_MAX_LENGTH: usize = 255;

// ---- Archive kind list (observed, 0x1001e0d60) ----------------------------
pub const ARCHIVE_KIND_EXTENSIONS: &[&str] = &["zip", "tar", "gz", "7z", "rar", "xz", "zst", "tgz", "tbz", "cab"];

// ---- Document kind lists used by the `kind` condition (derived) ------------
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "heic", "raw"];
pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "avi", "mkv", "webm"];
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "aac", "wav", "flac", "aiff", "ogg"];
pub const DOCUMENT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rtf", "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "csv", "odt", "ods", "odp", "pages", "numbers", "key", "epub",
];

/// (année, mois, jour, heure, minute, seconde) d'un instant Unix, à l'heure locale du système
/// (fuseau et heure d'été compris).
pub fn local_parts(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    use chrono::{Datelike, Local, TimeZone, Timelike};
    match Local.timestamp_opt(secs, 0).single() {
        Some(t) => (t.year() as i64, t.month(), t.day(), t.hour(), t.minute(), t.second()),
        None => parts_at_offset(secs, 0),
    }
}

/// Même découpage pour un décalage fixe en secondes par rapport à UTC.
pub fn parts_at_offset(secs: i64, offset_secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let local = secs + offset_secs;
    let (y, mo, d) = civil_from_days(local.div_euclid(86_400));
    let tod = local.rem_euclid(86_400);
    (y, mo, d, (tod / 3600) as u32, ((tod / 60) % 60) as u32, (tod % 60) as u32)
}

/// Calendar days -> (year, month, day), proleptic Gregorian.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Size values entered in the rule editor: "10 MB", "1.5gb", "300" (bytes).
pub fn parse_size(s: &str) -> Option<u64> {
    let t = s.trim().to_lowercase();
    if t.is_empty() {
        return None;
    }
    let (num, mult) = if let Some(rest) = t.strip_suffix("kb") {
        (rest, 1024u64)
    } else if let Some(rest) = t.strip_suffix("mb") {
        (rest, 1024 * 1024)
    } else if let Some(rest) = t.strip_suffix("gb") {
        (rest, 1024 * 1024 * 1024)
    } else if let Some(rest) = t.strip_suffix("tb") {
        (rest, 1024 * 1024 * 1024 * 1024)
    } else if let Some(rest) = t.strip_suffix('b') {
        (rest, 1)
    } else {
        (t.as_str(), 1)
    };
    let num = num.trim();
    if num.is_empty() {
        return None;
    }
    if let Ok(v) = num.parse::<f64>() {
        Some((v * mult as f64).round() as u64)
    } else {
        None
    }
}

/// "30 days" / "2 weeks" / "1 month" / "1 year" -> seconds (for `within_last`).
pub fn parse_span(s: &str) -> Option<u64> {
    let t = s.trim().to_lowercase();
    let parts: Vec<&str> = t.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let n: f64 = parts[0].parse().ok()?;
    let unit = parts.get(1).copied().unwrap_or("days");
    let secs = match unit.trim_end_matches('s') {
        "day" => 86_400.0,
        "week" => 7.0 * 86_400.0,
        "month" => 30.0 * 86_400.0,
        "year" => 365.0 * 86_400.0,
        _ => 86_400.0,
    };
    Some((n * secs) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_parsing() {
        assert_eq!(parse_size("10 MB"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("512"), Some(512));
    }

    #[test]
    fn offset_shifts_the_day_and_the_hour() {
        // 2026-01-15 23:30:00 UTC
        let secs = 1_768_519_800;
        assert_eq!(parts_at_offset(secs, 0), (2026, 1, 15, 23, 30, 0));
        // Paris en hiver (UTC+1) : minuit passé, on est le lendemain.
        assert_eq!(parts_at_offset(secs, 3600), (2026, 1, 16, 0, 30, 0));
        // New York en hiver (UTC-5).
        assert_eq!(parts_at_offset(secs, -5 * 3600), (2026, 1, 15, 18, 30, 0));
    }

    #[test]
    fn local_parts_matches_the_system_offset() {
        use chrono::{Local, Offset, TimeZone};
        let secs = 1_768_519_800;
        let offset = Local.timestamp_opt(secs, 0).single().map(|t| t.offset().fix().local_minus_utc() as i64).unwrap_or(0);
        assert_eq!(local_parts(secs), parts_at_offset(secs, offset));
    }
}
