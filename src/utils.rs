use chrono::{DateTime, Utc, TimeZone};

/// Formats a yoctoNEAR balance into a human-readable NEAR string with 4 decimal places.
pub fn format_near(yocto: u128) -> String {
    format!("{:.4} NEAR", yocto as f64 / crate::near::YOCTO_NEAR)
}

/// Returns the current UTC time formatted as YYYY-MM-DD HH:MM:SS.
pub fn now_timestamp() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Formats a nanosecond timestamp string into a human-readable UTC date and time.
pub fn format_timestamp(ns_str: String) -> String {
    let ns = ns_str.parse::<u128>().unwrap_or(0);
    let secs = (ns / 1_000_000_000) as i64;
    let nsecs = (ns % 1_000_000_000) as u32;
    
    match Utc.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        _ => "Invalid Timestamp".to_string(),
    }
}
