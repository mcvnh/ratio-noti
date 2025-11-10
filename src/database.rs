use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, sqlite::SqlitePool};

pub struct Database {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct RatioRecord {
    pub id: i64,
    pub pair_name: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub price_a: f64,
    pub price_b: f64,
    pub ratio: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AlertRecord {
    pub id: i64,
    pub pair_name: String,
    pub ratio: f64,
    pub change_percentage: f64,
    pub threshold: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VolumeRatioRecord {
    pub id: i64,
    pub pair_name: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub volume: f64,
    pub effective_price_a: f64,
    pub effective_price_b: f64,
    pub ratio: f64,
    pub slippage_a: f64,
    pub slippage_b: f64,
    pub timestamp: DateTime<Utc>,
}

impl Database {
    /// Create a new database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url)
            .await
            .context("Failed to connect to database")?;

        let db = Self { pool };
        db.init_schema().await?;

        Ok(db)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        // Create ratio_snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ratio_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pair_name TEXT NOT NULL,
                symbol_a TEXT NOT NULL,
                symbol_b TEXT NOT NULL,
                price_a REAL NOT NULL,
                price_b REAL NOT NULL,
                ratio REAL NOT NULL,
                timestamp TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create ratio_snapshots table")?;

        // Create index on pair_name and timestamp
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_ratio_snapshots_pair_timestamp
            ON ratio_snapshots(pair_name, timestamp DESC)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create index")?;

        // Create alerts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pair_name TEXT NOT NULL,
                ratio REAL NOT NULL,
                change_percentage REAL NOT NULL,
                threshold REAL NOT NULL,
                timestamp TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create alerts table")?;

        // Create index on alerts
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_alerts_pair_timestamp
            ON alerts(pair_name, timestamp DESC)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create alerts index")?;

        // Create volume_ratios table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS volume_ratios (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pair_name TEXT NOT NULL,
                symbol_a TEXT NOT NULL,
                symbol_b TEXT NOT NULL,
                volume REAL NOT NULL,
                effective_price_a REAL NOT NULL,
                effective_price_b REAL NOT NULL,
                ratio REAL NOT NULL,
                slippage_a REAL NOT NULL,
                slippage_b REAL NOT NULL,
                timestamp TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create volume_ratios table")?;

        log::info!("Database schema initialized");

        Ok(())
    }

    /// Insert a ratio snapshot
    pub async fn insert_ratio_snapshot(
        &self,
        pair_name: &str,
        symbol_a: &str,
        symbol_b: &str,
        price_a: f64,
        price_b: f64,
        ratio: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO ratio_snapshots (pair_name, symbol_a, symbol_b, price_a, price_b, ratio, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(pair_name)
        .bind(symbol_a)
        .bind(symbol_b)
        .bind(price_a)
        .bind(price_b)
        .bind(ratio)
        .bind(timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to insert ratio snapshot")?;

        Ok(result.last_insert_rowid())
    }

    /// Insert an alert record
    pub async fn insert_alert(
        &self,
        pair_name: &str,
        ratio: f64,
        change_percentage: f64,
        threshold: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO alerts (pair_name, ratio, change_percentage, threshold, timestamp)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(pair_name)
        .bind(ratio)
        .bind(change_percentage)
        .bind(threshold)
        .bind(timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to insert alert")?;

        Ok(result.last_insert_rowid())
    }

    /// Insert a volume-based ratio record
    pub async fn insert_volume_ratio(
        &self,
        pair_name: &str,
        symbol_a: &str,
        symbol_b: &str,
        volume: f64,
        effective_price_a: f64,
        effective_price_b: f64,
        ratio: f64,
        slippage_a: f64,
        slippage_b: f64,
        timestamp: DateTime<Utc>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO volume_ratios (pair_name, symbol_a, symbol_b, volume,
                                      effective_price_a, effective_price_b, ratio,
                                      slippage_a, slippage_b, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(pair_name)
        .bind(symbol_a)
        .bind(symbol_b)
        .bind(volume)
        .bind(effective_price_a)
        .bind(effective_price_b)
        .bind(ratio)
        .bind(slippage_a)
        .bind(slippage_b)
        .bind(timestamp.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to insert volume ratio")?;

        Ok(result.last_insert_rowid())
    }

    /// Get ratio history for a specific pair
    pub async fn get_ratio_history(&self, pair_name: &str, limit: i64) -> Result<Vec<RatioRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, pair_name, symbol_a, symbol_b, price_a, price_b, ratio, timestamp
            FROM ratio_snapshots
            WHERE pair_name = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(pair_name)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch ratio history")?;

        let mut records = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .context("Failed to parse timestamp")?
                .with_timezone(&Utc);

            records.push(RatioRecord {
                id: row.get("id"),
                pair_name: row.get("pair_name"),
                symbol_a: row.get("symbol_a"),
                symbol_b: row.get("symbol_b"),
                price_a: row.get("price_a"),
                price_b: row.get("price_b"),
                ratio: row.get("ratio"),
                timestamp,
            });
        }

        Ok(records)
    }

    /// Get ratio history within a time range
    pub async fn get_ratio_history_range(
        &self,
        pair_name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<RatioRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, pair_name, symbol_a, symbol_b, price_a, price_b, ratio, timestamp
            FROM ratio_snapshots
            WHERE pair_name = ? AND timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(pair_name)
        .bind(start.to_rfc3339())
        .bind(end.to_rfc3339())
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch ratio history range")?;

        let mut records = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .context("Failed to parse timestamp")?
                .with_timezone(&Utc);

            records.push(RatioRecord {
                id: row.get("id"),
                pair_name: row.get("pair_name"),
                symbol_a: row.get("symbol_a"),
                symbol_b: row.get("symbol_b"),
                price_a: row.get("price_a"),
                price_b: row.get("price_b"),
                ratio: row.get("ratio"),
                timestamp,
            });
        }

        Ok(records)
    }

    /// Get alert history for a specific pair
    pub async fn get_alert_history(&self, pair_name: &str, limit: i64) -> Result<Vec<AlertRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, pair_name, ratio, change_percentage, threshold, timestamp
            FROM alerts
            WHERE pair_name = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(pair_name)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch alert history")?;

        let mut records = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .context("Failed to parse timestamp")?
                .with_timezone(&Utc);

            records.push(AlertRecord {
                id: row.get("id"),
                pair_name: row.get("pair_name"),
                ratio: row.get("ratio"),
                change_percentage: row.get("change_percentage"),
                threshold: row.get("threshold"),
                timestamp,
            });
        }

        Ok(records)
    }

    /// Get all alerts
    pub async fn get_all_alerts(&self, limit: i64) -> Result<Vec<AlertRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, pair_name, ratio, change_percentage, threshold, timestamp
            FROM alerts
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch all alerts")?;

        let mut records = Vec::new();
        for row in rows {
            let timestamp_str: String = row.get("timestamp");
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .context("Failed to parse timestamp")?
                .with_timezone(&Utc);

            records.push(AlertRecord {
                id: row.get("id"),
                pair_name: row.get("pair_name"),
                ratio: row.get("ratio"),
                change_percentage: row.get("change_percentage"),
                threshold: row.get("threshold"),
                timestamp,
            });
        }

        Ok(records)
    }

    /// Get statistics for a pair
    pub async fn get_pair_statistics(&self, pair_name: &str, hours: i64) -> Result<PairStatistics> {
        let since = Utc::now() - chrono::Duration::hours(hours);

        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as count,
                MIN(ratio) as min_ratio,
                MAX(ratio) as max_ratio,
                AVG(ratio) as avg_ratio
            FROM ratio_snapshots
            WHERE pair_name = ? AND timestamp >= ?
            "#,
        )
        .bind(pair_name)
        .bind(since.to_rfc3339())
        .fetch_one(&self.pool)
        .await
        .context("Failed to fetch statistics")?;

        Ok(PairStatistics {
            pair_name: pair_name.to_string(),
            count: row.get("count"),
            min_ratio: row.get("min_ratio"),
            max_ratio: row.get("max_ratio"),
            avg_ratio: row.get("avg_ratio"),
            hours,
        })
    }

    /// Clean up old records (older than specified days)
    pub async fn cleanup_old_records(&self, days: i64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days);

        let result = sqlx::query(
            r#"
            DELETE FROM ratio_snapshots WHERE timestamp < ?
            "#,
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to clean up old ratio snapshots")?;

        let deleted_ratios = result.rows_affected();

        let result = sqlx::query(
            r#"
            DELETE FROM alerts WHERE timestamp < ?
            "#,
        )
        .bind(cutoff.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to clean up old alerts")?;

        let deleted_alerts = result.rows_affected();

        log::info!(
            "Cleaned up {} ratio snapshots and {} alerts older than {} days",
            deleted_ratios,
            deleted_alerts,
            days
        );

        Ok(deleted_ratios + deleted_alerts)
    }
}

#[derive(Debug)]
pub struct PairStatistics {
    pub pair_name: String,
    pub count: i64,
    pub min_ratio: f64,
    pub max_ratio: f64,
    pub avg_ratio: f64,
    pub hours: i64,
}

impl PairStatistics {
    pub fn format_summary(&self) -> String {
        format!(
            "{} (last {} hours):\n  \
            Samples: {}\n  \
            Min: {:.8}\n  \
            Max: {:.8}\n  \
            Avg: {:.8}\n  \
            Range: {:.2}%",
            self.pair_name,
            self.hours,
            self.count,
            self.min_ratio,
            self.max_ratio,
            self.avg_ratio,
            ((self.max_ratio - self.min_ratio) / self.min_ratio * 100.0)
        )
    }
}
