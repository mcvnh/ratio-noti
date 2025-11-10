mod binance;
mod config;
mod monitor;
mod ratio;
mod telegram;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use binance::BinanceClient;
use config::Config;
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

    /// Test Telegram connection
    TestTelegram,

    /// Show all configured ratio pairs
    ListPairs,
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
        Commands::TestTelegram => {
            handle_test_telegram(&cli.config).await?;
        }
        Commands::ListPairs => {
            handle_list_pairs(&cli.config).await?;
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

    let client = BinanceClient::new();
    let calculator = RatioCalculator::new(client);
    let notifier = TelegramNotifier::new(&config.telegram.token, config.telegram.user_id);

    let mut monitor = RatioMonitor::new(config, calculator, notifier);

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
