use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub monitoring: MonitoringConfig,
    pub database: DatabaseConfig,
    pub ratio_pairs: Vec<RatioPair>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Path to SQLite database file
    pub path: String,
    /// Days to keep historical data (older data will be cleaned up)
    pub retention_days: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramConfig {
    pub token: String,
    pub user_id: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitoringConfig {
    /// Interval in seconds to check for ratio changes
    pub check_interval_secs: u64,
    /// Interval in seconds for periodic notifications (default: 3600 = 1 hour)
    pub periodic_notification_secs: u64,
    /// Thresholds for ratio change alerts (e.g., [5.0, 10.0, 15.0, 20.0] for 5%, 10%, 15%, 20%)
    pub change_thresholds: Vec<f64>,
    /// Time window in seconds to detect sudden changes (default: 300 = 5 minutes)
    pub change_window_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RatioPair {
    /// Name/identifier for this ratio pair
    pub name: String,
    /// First symbol (e.g., "BTCUSDT")
    pub symbol_a: String,
    /// Second symbol (e.g., "ETHUSDT")
    pub symbol_b: String,
    /// Volume in base currency for slippage analysis (optional)
    pub analysis_volume: Option<f64>,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {}", path))?;

        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.telegram.token.is_empty() {
            anyhow::bail!("Telegram token cannot be empty");
        }

        if self.ratio_pairs.is_empty() {
            anyhow::bail!("At least one ratio pair must be configured");
        }

        for pair in &self.ratio_pairs {
            if pair.symbol_a.is_empty() || pair.symbol_b.is_empty() {
                anyhow::bail!("Symbols cannot be empty in ratio pair: {}", pair.name);
            }
        }

        Ok(())
    }
}
