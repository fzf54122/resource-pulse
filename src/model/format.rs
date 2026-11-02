// @Time    : 2026/11/1 23:34
// @Author  : fzf
// @FileName: format.rs
// @Software: resource-pulse

pub fn ratio(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.
    } else {
        (used as f64 / total as f64).clamp(0., 1.) as f32
    }
}

pub fn bytes(value: u64) -> String {
    let (divisor, unit) = if value >= 1024_u64.pow(4) {
        (1024_f64.powi(4), "TiB")
    } else if value >= 1024_u64.pow(3) {
        (1024_f64.powi(3), "GiB")
    } else if value >= 1024_u64.pow(2) {
        (1024_f64.powi(2), "MiB")
    } else if value >= 1024 {
        (1024., "KiB")
    } else {
        return format!("{value} B");
    };
    format!("{:.1} {unit}", value as f64 / divisor)
}

pub fn rate(kib: f32) -> String {
    let kib = if kib.is_finite() { kib.max(0.) } else { 0. };
    if kib >= 1024. {
        format!("{:.1} MiB/s", kib / 1024.)
    } else {
        format!("{kib:.1} KiB/s")
    }
}

pub fn duration(seconds: u64) -> String {
    if seconds >= 86400 {
        format!("{} 天 {} 小时", seconds / 86400, seconds / 3600 % 24)
    } else if seconds >= 3600 {
        format!("{} 小时 {} 分钟", seconds / 3600, seconds / 60 % 60)
    } else {
        format!("{} 分 {} 秒", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_preserve_small_values_and_scale_large_values() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1024), "1.0 KiB");
        assert_eq!(bytes(1024_u64.pow(4)), "1.0 TiB");
        assert_eq!(rate(1024.), "1.0 MiB/s");
        assert_eq!(rate(f32::NAN), "0.0 KiB/s");
        assert_eq!(rate(-10.), "0.0 KiB/s");
    }

    #[test]
    fn uptime_includes_minutes_for_recent_boots() {
        assert_eq!(duration(61), "1 分 1 秒");
        assert_eq!(duration(3660), "1 小时 1 分钟");
        assert_eq!(duration(90000), "1 天 1 小时");
    }
}
