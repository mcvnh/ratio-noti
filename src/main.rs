mod binance;
mod bot;
mod config;
mod database;
mod monitor;
mod ratio;
mod telegram;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use binance::BinanceClient;
use bot::BotHandler;
use config::Config;
use database::Database;
use monitor::RatioMonitor;
use ratio::{OrderSide, RatioCalculator};
use telegram::TelegramNotifier;

#[derive(Parser)]
#[command(name = "ratio-noti")]
#[command(about = "Cryptocurrency price ratio calculator and monitoring tool", long_about = None)]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Calculate simple price ratio
    Simple {
        /// Name for the ratio pair
        #[arg(short, long)]
        name: String,

        /// First symbol (e.g., BTCUSDT)
        #[arg(short = 'a', long)]
        symbol_a: String,

        /// Second symbol (e.g., ETHUSDT)
        #[arg(short = 'b', long)]
        symbol_b: String,
    },

    /// Calculate volume-based ratio with order book analysis
    Volume {
        /// Name for the ratio pair
        #[arg(short, long)]
        name: String,

        /// First symbol (e.g., BTCUSDT)
        #[arg(short = 'a', long)]
        symbol_a: String,

        /// Second symbol (e.g., ETHUSDT)
        #[arg(short = 'b', long)]
        symbol_b: String,

        /// Volume for analysis
        #[arg(short, long)]
        volume: f64,
    },

    /// Analyze slippage for a specific trade
    Slippage {
        /// Symbol to analyze (e.g., BTCUSDT)
        #[arg(short, long)]
        symbol: String,

        /// Volume to analyze
        #[arg(short, long)]
        volume: f64,

        /// Order side (buy or sell)
        #[arg(short = 's', long, default_value = "buy")]
        side: String,
    },

    /// Start monitoring ratios (uses config file)
    Monitor,

    /// Start interactive Telegram bot with buttons
    Bot,

    /// Test Telegram connection
    TestTelegram,

    /// Show all configured ratio pairs
    ListPairs,

    /// Query historical ratio data
    History {
        /// Pair name to query
        #[arg(short, long)]
        pair: String,

        /// Number of records to show (default: 100)
        #[arg(short, long, default_value = "100")]
        limit: i64,
    },

    /// Show alert history
    Alerts {
        /// Optional pair name to filter alerts
        #[arg(short, long)]
        pair: Option<String>,

        /// Number of alerts to show (default: 50)
        #[arg(short, long, default_value = "50")]
        limit: i64,
    },

    /// Show statistics for a pair
    Stats {
        /// Pair name
        #[arg(short, long)]
        pair: String,

        /// Number of hours to analyze (default: 24)
        #[arg(long, default_value = "24")]
        hours: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Simple {
            name,
            symbol_a,
            symbol_b,
        } => {
            handle_simple_ratio(&name, &symbol_a, &symbol_b).await?;
        }
        Commands::Volume {
            name,
            symbol_a,
            symbol_b,
            volume,
        } => {
            handle_volume_ratio(&name, &symbol_a, &symbol_b, volume).await?;
        }
        Commands::Slippage {
            symbol,
            volume,
            side,
        } => {
            handle_slippage(&symbol, volume, &side).await?;
        }
        Commands::Monitor => {
            handle_monitor(&cli.config).await?;
        }
        Commands::Bot => {
            handle_bot(&cli.config).await?;
        }
        Commands::TestTelegram => {
            handle_test_telegram(&cli.config).await?;
        }
        Commands::ListPairs => {
            handle_list_pairs(&cli.config).await?;
        }
        Commands::History { pair, limit } => {
            handle_history(&cli.config, &pair, limit).await?;
        }
        Commands::Alerts { pair, limit } => {
            handle_alerts(&cli.config, pair.as_deref(), limit).await?;
        }
        Commands::Stats { pair, hours } => {
            handle_stats(&cli.config, &pair, hours).await?;
        }
    }

    Ok(())
}

async fn handle_simple_ratio(name: &str, symbol_a: &str, symbol_b: &str) -> Result<()> {
    log::info!("Calculating simple ratio for {} / {}", symbol_a, symbol_b);

    let client = BinanceClient::new();
    let calculator = RatioCalculator::new(client);

    let ratio = calculator
        .calculate_simple_ratio(name, symbol_a, symbol_b)
        .await?;

    println!("\n{}", "=".repeat(60));
    println!("Simple Price Ratio");
    println!("{}", "=".repeat(60));
    println!("{}", ratio.format_summary());
    println!("Timestamp: {}", ratio.timestamp);
    println!("{}", "=".repeat(60));

    Ok(())
}

async fn handle_volume_ratio(
    name: &str,
    symbol_a: &str,
    symbol_b: &str,
    volume: f64,
) -> Result<()> {
    log::info!(
        "Calculating volume-based ratio for {} / {} with volume {}",
        symbol_a,
        symbol_b,
        volume
    );

    let client = BinanceClient::new();
    let calculator = RatioCalculator::new(client);

    let ratio = calculator
        .calculate_volume_based_ratio(name, symbol_a, symbol_b, volume)
        .await?;

    println!("\n{}", "=".repeat(60));
    println!("Volume-Based Ratio (with Order Book Analysis)");
    println!("{}", "=".repeat(60));
    println!("{}", ratio.format_summary());
    println!("Timestamp: {}", ratio.timestamp);
    println!("{}", "=".repeat(60));

    Ok(())
}

async fn handle_slippage(symbol: &str, volume: f64, side: &str) -> Result<()> {
    log::info!("Analyzing slippage for {} {} {}", side, volume, symbol);

    let order_side = match side.to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => anyhow::bail!("Invalid side: {}. Must be 'buy' or 'sell'", side),
    };

    let client = BinanceClient::new();
    let calculator = RatioCalculator::new(client);

    let analysis = calculator
        .analyze_slippage(symbol, volume, order_side)
        .await?;

    println!("\n{}", "=".repeat(60));
    println!("Slippage Analysis");
    println!("{}", "=".repeat(60));
    println!("{}", analysis.format_summary());
    println!("{}", "=".repeat(60));

    Ok(())
}

async fn handle_monitor(config_path: &str) -> Result<()> {
    log::info!("Loading configuration from {}", config_path);

    let config = Config::from_file(config_path)
        .context("Failed to load config file. Did you create config.toml?")?;

    config.validate()?;

    log::info!("Configuration loaded successfully");
    log::info!("Monitoring {} ratio pairs", config.ratio_pairs.len());

    // Initialize database
    let db_url = format!("sqlite:{}?mode=rwc", config.database.path);
    let database = Database::new(&db_url)
        .await
        .context("Failed to initialize database")?;
    log::info!("Database initialized at {}", config.database.path);

    let client = BinanceClient::new();
    let calculator = RatioCalculator::new(client);
    let notifier = TelegramNotifier::new(&config.telegram.token, config.telegram.user_id);

    let mut monitor = RatioMonitor::new(config, calculator, notifier, database);

    monitor.start().await?;

    Ok(())
}

async fn handle_test_telegram(config_path: &str) -> Result<()> {
    log::info!("Testing Telegram connection...");

    let config = Config::from_file(config_path)
        .context("Failed to load config file. Did you create config.toml?")?;

    let notifier = TelegramNotifier::new(&config.telegram.token, config.telegram.user_id);

    notifier.test_connection().await?;

    println!("✅ Telegram connection successful!");

    Ok(())
}

async fn handle_bot(config_path: &str) -> Result<()> {
    log::info!("Starting interactive Telegram bot...");

    let config = Config::from_file(config_path)
        .context("Failed to load config file. Did you create config.toml?")?;

    config.validate()?;

    log::info!("Configuration loaded successfully");
    log::info!("Bot configured with {} ratio pairs", config.ratio_pairs.len());

    let client = BinanceClient::new();
    let calculator = RatioCalculator::new(client);

    let bot_handler = BotHandler::new(config, calculator);

    println!("\n{}", "=".repeat(60));
    println!("Interactive Telegram Bot Started");
    println!("{}", "=".repeat(60));
    println!("Open your Telegram app and send /start to the bot");
    println!("Press Ctrl+C to stop");
    println!("{}", "=".repeat(60));

    bot_handler.run().await?;

    Ok(())
}

async fn handle_list_pairs(config_path: &str) -> Result<()> {
    let config = Config::from_file(config_path)
        .context("Failed to load config file. Did you create config.toml?")?;

    println!("\n{}", "=".repeat(60));
    println!("Configured Ratio Pairs");
    println!("{}", "=".repeat(60));

    for (i, pair) in config.ratio_pairs.iter().enumerate() {
        println!("\n{}. {}", i + 1, pair.name);
        println!("   Symbol A: {}", pair.symbol_a);
        println!("   Symbol B: {}", pair.symbol_b);
        if let Some(vol) = pair.analysis_volume {
            println!("   Analysis Volume: {}", vol);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Total pairs: {}", config.ratio_pairs.len());
    println!("{}", "=".repeat(60));

    Ok(())
}

async fn handle_history(config_path: &str, pair_name: &str, limit: i64) -> Result<()> {
    let config = Config::from_file(config_path)
        .context("Failed to load config file")?;

    let db_url = format!("sqlite:{}?mode=rwc", config.database.path);
    let database = Database::new(&db_url).await?;

    let records = database.get_ratio_history(pair_name, limit).await?;

    println!("\n{}", "=".repeat(60));
    println!("Ratio History: {}", pair_name);
    println!("{}", "=".repeat(60));

    if records.is_empty() {
        println!("No historical data found for {}", pair_name);
    } else {
        for record in &records {
            println!(
                "{} | Ratio: {:.8} | {} ${:.2} / {} ${:.2}",
                record.timestamp.format("%Y-%m-%d %H:%M:%S"),
                record.ratio,
                record.symbol_a,
                record.price_a,
                record.symbol_b,
                record.price_b
            );
        }
        println!("\nTotal records: {}", records.len());
    }

    println!("{}", "=".repeat(60));

    Ok(())
}

async fn handle_alerts(config_path: &str, pair_name: Option<&str>, limit: i64) -> Result<()> {
    let config = Config::from_file(config_path)
        .context("Failed to load config file")?;

    let db_url = format!("sqlite:{}?mode=rwc", config.database.path);
    let database = Database::new(&db_url).await?;

    let records = if let Some(pair) = pair_name {
        database.get_alert_history(pair, limit).await?
    } else {
        database.get_all_alerts(limit).await?
    };

    println!("\n{}", "=".repeat(60));
    if let Some(pair) = pair_name {
        println!("Alert History: {}", pair);
    } else {
        println!("Alert History: All Pairs");
    }
    println!("{}", "=".repeat(60));

    if records.is_empty() {
        println!("No alerts found");
    } else {
        for alert in &records {
            println!(
                "{} | {} | Ratio: {:.8} | Change: {:+.2}% (threshold: {}%)",
                alert.timestamp.format("%Y-%m-%d %H:%M:%S"),
                alert.pair_name,
                alert.ratio,
                alert.change_percentage,
                alert.threshold
            );
        }
        println!("\nTotal alerts: {}", records.len());
    }

    println!("{}", "=".repeat(60));

    Ok(())
}

async fn handle_stats(config_path: &str, pair_name: &str, hours: i64) -> Result<()> {
    let config = Config::from_file(config_path)
        .context("Failed to load config file")?;

    let db_url = format!("sqlite:{}?mode=rwc", config.database.path);
    let database = Database::new(&db_url).await?;

    let stats = database.get_pair_statistics(pair_name, hours).await?;

    println!("\n{}", "=".repeat(60));
    println!("Statistics");
    println!("{}", "=".repeat(60));
    println!("{}", stats.format_summary());
    println!("{}", "=".repeat(60));

    Ok(())
}
