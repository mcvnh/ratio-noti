use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::{interval, Instant};

use crate::config::{Config, RatioPair};
use crate::ratio::{RatioCalculator, SimpleRatio};
use crate::telegram::TelegramNotifier;

#[derive(Debug, Clone)]
struct RatioSnapshot {
    ratio: f64,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub struct RatioMonitor {
    config: Config,
    calculator: RatioCalculator,
    notifier: TelegramNotifier,
    history: HashMap<String, Vec<RatioSnapshot>>,
    last_periodic_notification: Instant,
    triggered_thresholds: HashMap<String, Vec<f64>>,
}

impl RatioMonitor {
    pub fn new(
        config: Config,
        calculator: RatioCalculator,
        notifier: TelegramNotifier,
    ) -> Self {
        Self {
            config,
            calculator,
            notifier,
            history: HashMap::new(),
            last_periodic_notification: Instant::now(),
            triggered_thresholds: HashMap::new(),
        }
    }

    /// Start monitoring ratios
    pub async fn start(&mut self) -> Result<()> {
        log::info!("Starting ratio monitor...");
        log::info!("Monitoring {} pairs", self.config.ratio_pairs.len());

        // Send initial connection test
        self.notifier.test_connection().await?;

        let mut check_interval = interval(Duration::from_secs(
            self.config.monitoring.check_interval_secs,
        ));

        loop {
            check_interval.tick().await;

            if let Err(e) = self.check_ratios().await {
                log::error!("Error checking ratios: {}", e);
            }

            if let Err(e) = self.check_periodic_notification().await {
                log::error!("Error sending periodic notification: {}", e);
            }
        }
    }

    /// Check all configured ratio pairs
    async fn check_ratios(&mut self) -> Result<()> {
        let pairs = self.config.ratio_pairs.clone();
        for pair in &pairs {
            if let Err(e) = self.check_ratio_pair(pair).await {
                log::error!("Error checking pair {}: {}", pair.name, e);
            }
        }
        Ok(())
    }

    /// Check a single ratio pair
    async fn check_ratio_pair(&mut self, pair: &RatioPair) -> Result<()> {
        // Calculate current ratio
        let ratio_data = self
            .calculator
            .calculate_simple_ratio(&pair.name, &pair.symbol_a, &pair.symbol_b)
            .await?;

        log::debug!("Checked {}: ratio = {:.8}", pair.name, ratio_data.ratio);

        // Store in history
        self.add_to_history(&pair.name, &ratio_data);

        // Check for threshold breaches
        self.check_thresholds(&pair.name, &ratio_data).await?;

        Ok(())
    }

    /// Add ratio to history
    fn add_to_history(&mut self, pair_name: &str, ratio_data: &SimpleRatio) {
        let snapshot = RatioSnapshot {
            ratio: ratio_data.ratio,
            timestamp: ratio_data.timestamp,
        };

        let history = self.history.entry(pair_name.to_string()).or_insert_with(Vec::new);
        history.push(snapshot);

        // Keep history within the time window (plus some buffer)
        let cutoff_time = chrono::Utc::now()
            - chrono::Duration::seconds((self.config.monitoring.change_window_secs * 2) as i64);

        history.retain(|s| s.timestamp > cutoff_time);
    }

    /// Check if any thresholds are breached
    async fn check_thresholds(&mut self, pair_name: &str, current: &SimpleRatio) -> Result<()> {
        let history = match self.history.get(pair_name) {
            Some(h) => h,
            None => return Ok(()),
        };

        let window_start = chrono::Utc::now()
            - chrono::Duration::seconds(self.config.monitoring.change_window_secs as i64);

        // Find the oldest snapshot within the time window
        let baseline = history
            .iter()
            .find(|s| s.timestamp >= window_start)
            .or_else(|| history.first());

        let baseline = match baseline {
            Some(b) => b,
            None => return Ok(()),
        };

        // Calculate percentage change
        let change_pct = ((current.ratio - baseline.ratio) / baseline.ratio) * 100.0;
        let abs_change = change_pct.abs();

        // Check each threshold
        let thresholds = self.config.monitoring.change_thresholds.clone();
        for threshold in thresholds {
            if abs_change >= threshold {
                // Check if we've already alerted for this threshold recently
                if !self.was_threshold_recently_triggered(pair_name, threshold) {
                    log::info!(
                        "Threshold breach for {}: {:.2}% change (threshold: {}%)",
                        pair_name,
                        change_pct,
                        threshold
                    );

                    let time_window = format_duration(self.config.monitoring.change_window_secs);

                    self.notifier
                        .send_ratio_alert(pair_name, current.ratio, change_pct, &time_window)
                        .await?;

                    self.mark_threshold_triggered(pair_name, threshold);
                }
            }
        }

        Ok(())
    }

    /// Check if threshold was recently triggered
    fn was_threshold_recently_triggered(&self, pair_name: &str, threshold: f64) -> bool {
        self.triggered_thresholds
            .get(pair_name)
            .map(|thresholds| thresholds.contains(&threshold))
            .unwrap_or(false)
    }

    /// Mark threshold as triggered
    fn mark_threshold_triggered(&mut self, pair_name: &str, threshold: f64) {
        let thresholds = self
            .triggered_thresholds
            .entry(pair_name.to_string())
            .or_insert_with(Vec::new);

        if !thresholds.contains(&threshold) {
            thresholds.push(threshold);
        }

        // Reset triggered thresholds after 2x the change window
        // This is handled by clearing old history
    }

    /// Reset triggered thresholds for a pair (called when ratio stabilizes)
    fn reset_triggered_thresholds(&mut self, pair_name: &str) {
        self.triggered_thresholds.remove(pair_name);
    }

    /// Check if it's time for periodic notification
    async fn check_periodic_notification(&mut self) -> Result<()> {
        let elapsed = self.last_periodic_notification.elapsed();
        let period = Duration::from_secs(self.config.monitoring.periodic_notification_secs);

        if elapsed >= period {
            self.send_periodic_notification().await?;
            self.last_periodic_notification = Instant::now();

            // Reset triggered thresholds on periodic notifications
            let pairs = self.config.ratio_pairs.clone();
            for pair in &pairs {
                self.reset_triggered_thresholds(&pair.name);
            }
        }

        Ok(())
    }

    /// Send periodic notification with all current ratios
    async fn send_periodic_notification(&self) -> Result<()> {
        log::info!("Sending periodic notification");

        let mut updates = Vec::new();

        for pair in &self.config.ratio_pairs {
            match self
                .calculator
                .calculate_simple_ratio(&pair.name, &pair.symbol_a, &pair.symbol_b)
                .await
            {
                Ok(ratio) => {
                    let update = format!(
                        "*{}*\n`{:.8}`\n{} ${:.2} / {} ${:.2}",
                        pair.name,
                        ratio.ratio,
                        pair.symbol_a,
                        ratio.price_a,
                        pair.symbol_b,
                        ratio.price_b
                    );
                    updates.push(update);
                }
                Err(e) => {
                    log::error!("Failed to calculate ratio for {}: {}", pair.name, e);
                }
            }
        }

        if !updates.is_empty() {
            self.notifier.send_periodic_update(&updates).await?;
        }

        Ok(())
    }
}

/// Format duration in seconds to human-readable string
fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h", seconds / 3600)
    }
}
