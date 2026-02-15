/// Formats a yoctoNEAR balance into a human-readable NEAR string with 4 decimal places.
pub fn format_near(yocto: u128) -> String {
    format!("{:.4} NEAR", yocto as f64 / crate::near::YOCTO_NEAR)
}

/// Returns the current UTC time formatted as HH:MM:SS.
pub fn now_timestamp() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02} UTC")
}
