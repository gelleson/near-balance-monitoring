//! Utility functions for formatting balances and timestamps.
//!
//! This module provides helper functions for converting between different
//! representations of NEAR balances and timestamps.

use chrono::{Local, Utc, TimeZone};

/// Formats a yoctoNEAR balance into a human-readable NEAR string.
///
/// Converts yoctoNEAR (10^24 yoctoNEAR = 1 NEAR) to NEAR with 4 decimal places.
///
/// # Arguments
///
/// * `yocto` - Balance in yoctoNEAR
///
/// # Returns
///
/// A string formatted as "X.XXXX NEAR"
///
/// # Examples
///
/// ```
/// # use near_balance_monitor::utils::format_near;
/// let balance = 1_500_000_000_000_000_000_000_000u128; // 1.5 NEAR
/// assert_eq!(format_near(balance), "1.5000 NEAR");
/// ```
pub fn format_near(yocto: u128) -> String {
    format!("{:.4} NEAR", yocto as f64 / crate::near::YOCTO_NEAR)
}

/// Returns the current local time as a formatted string.
///
/// # Returns
///
/// A string formatted as "YYYY-MM-DD HH:MM:SS TZ"
///
/// # Examples
///
/// ```
/// # use near_balance_monitor::utils::now_timestamp;
/// let timestamp = now_timestamp();
/// // Example output: "2026-02-15 10:30:45 PST"
/// ```
pub fn now_timestamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S %Z").to_string()
}

/// Formats a nanosecond timestamp string into a human-readable local date and time.
///
/// Converts a nanosecond timestamp (as used in NEAR block timestamps) into
/// a local timezone-aware string.
///
/// # Arguments
///
/// * `ns_str` - Timestamp in nanoseconds as a string
///
/// # Returns
///
/// A formatted string "YYYY-MM-DD HH:MM:SS TZ" or "Invalid Timestamp" if parsing fails.
///
/// # Examples
///
/// ```
/// # use near_balance_monitor::utils::format_timestamp;
/// let ns_timestamp = "1708000000000000000".to_string();
/// let formatted = format_timestamp(ns_timestamp);
/// // Example output: "2024-02-15 10:13:20 PST"
/// ```
pub fn format_timestamp(ns_str: String) -> String {
    let ns = match ns_str.parse::<u128>() {
        Ok(n) => n,
        Err(e) => {
            log::warn!("Failed to parse timestamp timestamp={}: {}", ns_str, e);
            return "Invalid Timestamp".to_string();
        }
    };
    let secs = (ns / 1_000_000_000) as i64;
    let nsecs = (ns % 1_000_000_000) as u32;

    match Utc.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => {
            let local_dt = dt.with_timezone(&Local);
            local_dt.format("%Y-%m-%d %H:%M:%S %Z").to_string()
        },
        _ => {
            log::warn!("Failed to convert timestamp secs={} nsecs={}", secs, nsecs);
            "Invalid Timestamp".to_string()
        }
    }
}
