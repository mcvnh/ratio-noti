# Ratio-Noti

A Rust CLI application for calculating and monitoring cryptocurrency price ratios using real-time data from Binance, with automatic Telegram notifications.

## Features

- **Simple Price Ratio**: Quick calculation using current market prices
- **Volume-Based Ratio**: Advanced calculation considering order book depth and slippage
- **Real-Time Monitoring**: Continuous monitoring with configurable check intervals
- **Smart Alerts**: Get notified when ratios change by 5%, 10%, 15%, 20%, or custom thresholds
- **Periodic Updates**: Receive hourly summary reports of all monitored ratios
- **Slippage Analysis**: Understand price impact for specific trade volumes
- **Telegram Integration**: Receive all notifications directly in Telegram
- **Interactive Bot**: Chat with your bot to get ratios on-demand with button menus
- **Persistent Storage**: SQLite database stores all historical ratio data and alerts
- **Statistics & Analysis**: Query historical data, view trends, and analyze price movements

## Quick Start

### Prerequisites

- Rust toolchain (2024 edition)
- A Telegram bot token (get from [@BotFather](https://t.me/BotFather))
- Your Telegram user ID (get from [@userinfobot](https://t.me/userinfobot))

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd ratio-noti
```

2. Build the project:
```bash
cargo build --release
```

3. Create your configuration file:
```bash
cp config.example.toml config.toml
```

4. Edit `config.toml` with your settings:
   - Add your Telegram bot token
   - Add your Telegram user ID
   - Configure the ratio pairs you want to monitor

5. Test your Telegram connection:
```bash
cargo run --release -- test-telegram
```

6. Start monitoring:
```bash
cargo run --release -- monitor
```

## Usage

### Monitor Mode (Continuous)

Start the monitoring service that will continuously check your configured ratios and send alerts:

```bash
cargo run --release -- monitor
```

This will:
- Monitor all ratio pairs defined in `config.toml`
- Send alerts when thresholds are breached (5%, 10%, 15%, 20%, etc.)
- Send periodic updates every hour
- Run continuously until stopped (Ctrl+C)

### One-Time Calculations

#### Simple Ratio
Calculate a quick ratio using current market prices:

```bash
cargo run --release -- simple \
  --name "BTC/ETH" \
  --symbol-a BTCUSDT \
  --symbol-b ETHUSDT
```

#### Volume-Based Ratio
Calculate ratio considering order book depth and slippage:

```bash
cargo run --release -- volume \
  --name "BTC/ETH" \
  --symbol-a BTCUSDT \
  --symbol-b ETHUSDT \
  --volume 1.0
```

#### Slippage Analysis
Analyze price impact for a specific trade:

```bash
cargo run --release -- slippage \
  --symbol BTCUSDT \
  --volume 1.0 \
  --side buy
```

### Historical Data & Statistics

Query historical ratio data:
```bash
cargo run --release -- history --pair "BTC/ETH" --limit 100
```

View alert history:
```bash
# All alerts
cargo run --release -- alerts --limit 50

# Filter by pair
cargo run --release -- alerts --pair "BTC/ETH" --limit 50
```

Show statistics:
```bash
cargo run --release -- stats --pair "BTC/ETH" --hours 24
```

### Utility Commands

List all configured ratio pairs:
```bash
cargo run --release -- list-pairs
```

Test Telegram connection:
```bash
cargo run --release -- test-telegram
```

## Configuration

Edit `config.toml` to customize your monitoring:

```toml
[telegram]
token = "YOUR_BOT_TOKEN"
user_id = 123456789

[database]
path = "ratio-noti.db"     # SQLite database file path
retention_days = 90         # Optional: auto-cleanup old data

[monitoring]
check_interval_secs = 60              # Check every 60 seconds
periodic_notification_secs = 3600      # Hourly updates
change_thresholds = [5.0, 10.0, 15.0, 20.0]  # Alert thresholds
change_window_secs = 300              # 5-minute window for change detection

[[ratio_pairs]]
name = "BTC/ETH"
symbol_a = "BTCUSDT"
symbol_b = "ETHUSDT"
analysis_volume = 1.0

[[ratio_pairs]]
name = "ETH/BNB"
symbol_a = "ETHUSDT"
symbol_b = "BNBUSDT"
analysis_volume = 10.0
```

### Configuration Parameters

- `check_interval_secs`: How often to check ratios (in seconds)
- `periodic_notification_secs`: How often to send summary updates (default: 3600 = 1 hour)
- `change_thresholds`: Percentage changes that trigger alerts (e.g., [5.0, 10.0, 15.0, 20.0])
- `change_window_secs`: Time window to detect sudden changes (default: 300 = 5 minutes)
- `retention_days`: Days to keep historical data (optional, default: keep all data)

## Data Persistence

All monitoring data is automatically stored in a SQLite database:

### What's Stored:
- **Ratio Snapshots**: Every ratio calculation with timestamp, prices, and calculated ratio
- **Alert History**: All triggered alerts with threshold information
- **Volume Analysis**: Results from volume-based ratio calculations (optional)

### Database Location:
- Default: `ratio-noti.db` in the working directory
- Configurable via `config.toml`
- Portable SQLite format (single file)

### Data Retention:
- Configure `retention_days` to automatically cleanup old data
- Example: `retention_days = 90` keeps last 90 days
- Omit or set to `null` to keep all data indefinitely

### Querying Data:
Use the CLI commands to access your historical data:
- `history` - View past ratio values
- `alerts` - Review all alerts
- `stats` - Analyze trends and statistics

## Example Notifications

### Threshold Alert
```
📈 Ratio Alert: BTC/ETH

Current Ratio: 0.05234567
Change: +5.23% in 5m
Time: 2025-11-10 15:30:00 UTC
```

### Periodic Update
```
📊 Periodic Ratio Update

BTC/ETH
0.05234567
BTCUSDT $43,250.00 / ETHUSDT $2,150.00

ETH/BNB
5.67891234
ETHUSDT $2,150.00 / BNBUSDT $378.50

Time: 2025-11-10 16:00:00 UTC
```

## Architecture

The application is built with a modular architecture:

- **binance.rs**: API client for Binance (prices and order books)
- **ratio.rs**: Ratio calculation engine (simple, volume-based, slippage)
- **monitor.rs**: Monitoring loop with threshold detection and database persistence
- **telegram.rs**: Telegram bot integration
- **bot.rs**: Interactive Telegram bot with button menus
- **database.rs**: SQLite storage for historical data and alerts
- **config.rs**: Configuration management

See [CLAUDE.md](CLAUDE.md) for detailed architecture documentation.

## Development

Run in debug mode:
```bash
cargo run -- monitor
```

Run tests:
```bash
cargo test
```

Run linter:
```bash
cargo clippy
```

Format code:
```bash
cargo fmt
```

## Logging

Set log level with `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run -- monitor  # Debug logging
RUST_LOG=info cargo run -- monitor   # Info logging (default)
```

## License

[Add your license here]

## Contributing

[Add contributing guidelines here]
