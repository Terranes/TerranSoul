use serde::Serialize;
use sysinfo::System;

/// Coarse RAM tier used for model recommendations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum RamTier {
    /// < 4 GB
    VeryLow,
    /// 4–8 GB
    Low,
    /// 8–16 GB
    Medium,
    /// 16–32 GB
    High,
    /// ≥ 32 GB
    VeryHigh,
}

impl RamTier {
    pub fn from_mb(total_mb: u64) -> Self {
        match total_mb {
            0..=4095 => RamTier::VeryLow,
            4096..=8191 => RamTier::Low,
            8192..=16383 => RamTier::Medium,
            16384..=32767 => RamTier::High,
            _ => RamTier::VeryHigh,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RamTier::VeryLow => "< 4 GB",
            RamTier::Low => "4–8 GB",
            RamTier::Medium => "8–16 GB",
            RamTier::High => "16–32 GB",
            RamTier::VeryHigh => "≥ 32 GB",
        }
    }
}

/// Hardware information collected from the host system.
#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    /// Total RAM in megabytes.
    pub total_ram_mb: u64,
    /// Human-readable RAM tier label (e.g. "8–16 GB").
    pub ram_tier_label: String,
    /// Number of logical CPU cores.
    pub cpu_cores: usize,
    /// CPU brand string (e.g. "Intel Core i7-12700H").
    pub cpu_name: String,
    /// OS name (e.g. "Windows 11", "macOS Sonoma", "Ubuntu 24.04").
    pub os_name: String,
    /// Architecture (e.g. "x86_64", "aarch64").
    pub arch: String,
}

/// Collect hardware information from the host system.
pub fn collect() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_ram_bytes = sys.total_memory();
    let total_ram_mb = total_ram_bytes / 1_048_576;

    let tier = RamTier::from_mb(total_ram_mb);
    let cpu_cores = sys.cpus().len();
    let cpu_name = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let os_name = format!(
        "{} {}",
        System::name().unwrap_or_else(|| "Unknown OS".to_string()),
        System::os_version().unwrap_or_default()
    )
    .trim()
    .to_string();

    let arch = std::env::consts::ARCH.to_string();

    SystemInfo {
        total_ram_mb,
        ram_tier_label: tier.label().to_string(),
        cpu_cores,
        cpu_name,
        os_name,
        arch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_tier_very_low() {
        assert_eq!(RamTier::from_mb(2048), RamTier::VeryLow);
        assert_eq!(RamTier::from_mb(0), RamTier::VeryLow);
        assert_eq!(RamTier::from_mb(4095), RamTier::VeryLow);
    }

    #[test]
    fn ram_tier_low() {
        assert_eq!(RamTier::from_mb(4096), RamTier::Low);
        assert_eq!(RamTier::from_mb(6000), RamTier::Low);
        assert_eq!(RamTier::from_mb(8191), RamTier::Low);
    }

    #[test]
    fn ram_tier_medium() {
        assert_eq!(RamTier::from_mb(8192), RamTier::Medium);
        assert_eq!(RamTier::from_mb(12000), RamTier::Medium);
        assert_eq!(RamTier::from_mb(16383), RamTier::Medium);
    }

    #[test]
    fn ram_tier_high() {
        assert_eq!(RamTier::from_mb(16384), RamTier::High);
        assert_eq!(RamTier::from_mb(24000), RamTier::High);
        assert_eq!(RamTier::from_mb(32767), RamTier::High);
    }

    #[test]
    fn ram_tier_very_high() {
        assert_eq!(RamTier::from_mb(32768), RamTier::VeryHigh);
        assert_eq!(RamTier::from_mb(65536), RamTier::VeryHigh);
    }

    #[test]
    fn tier_labels_are_non_empty() {
        for tier in [
            RamTier::VeryLow,
            RamTier::Low,
            RamTier::Medium,
            RamTier::High,
            RamTier::VeryHigh,
        ] {
            assert!(!tier.label().is_empty());
        }
    }

    #[test]
    fn collect_returns_sensible_values() {
        let info = collect();
        assert!(info.total_ram_mb > 0, "RAM must be positive");
        assert!(info.cpu_cores > 0, "CPU cores must be positive");
        assert!(!info.cpu_name.is_empty());
        assert!(!info.arch.is_empty());
    }
}
