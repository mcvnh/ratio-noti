use anyhow::Result;
use serde::Serialize;
use crate::binance::{BinanceClient, OrderBookInfo};

#[derive(Debug, Clone, Serialize)]
pub struct SimpleRatio {
    pub pair_name: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub price_a: f64,
    pub price_b: f64,
    pub ratio: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeBasedRatio {
    pub pair_name: String,
    pub symbol_a: String,
    pub symbol_b: String,
    pub volume: f64,
    pub effective_price_a: f64,
    pub effective_price_b: f64,
    pub ratio: f64,
    pub slippage_a: f64,
    pub slippage_b: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlippageAnalysis {
    pub symbol: String,
    pub mid_price: f64,
    pub volume: f64,
    pub side: OrderSide,
    pub effective_price: f64,
    pub slippage_percentage: f64,
    pub depth_consumed: usize,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
pub enum OrderSide {
    Buy,
    Sell,
}

pub struct RatioCalculator {
    client: BinanceClient,
}

impl RatioCalculator {
    pub fn new(client: BinanceClient) -> Self {
        Self { client }
    }

    /// Calculate simple ratio using current market prices
    pub async fn calculate_simple_ratio(
        &self,
        pair_name: &str,
        symbol_a: &str,
        symbol_b: &str,
    ) -> Result<SimpleRatio> {
        let price_a = self.client.get_price(symbol_a).await?;
        let price_b = self.client.get_price(symbol_b).await?;

        let ratio = price_a.price / price_b.price;

        Ok(SimpleRatio {
            pair_name: pair_name.to_string(),
            symbol_a: symbol_a.to_string(),
            symbol_b: symbol_b.to_string(),
            price_a: price_a.price,
            price_b: price_b.price,
            ratio,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Calculate volume-based ratio considering order book depth
    pub async fn calculate_volume_based_ratio(
        &self,
        pair_name: &str,
        symbol_a: &str,
        symbol_b: &str,
        volume: f64,
    ) -> Result<VolumeBasedRatio> {
        // Fetch order books
        let order_book_a = self.client.get_order_book(symbol_a, 100).await?;
        let order_book_b = self.client.get_order_book(symbol_b, 100).await?;

        // Calculate effective prices with slippage
        let (effective_price_a, slippage_a) =
            Self::calculate_effective_price(&order_book_a, volume, OrderSide::Buy)?;
        let (effective_price_b, slippage_b) =
            Self::calculate_effective_price(&order_book_b, volume, OrderSide::Buy)?;

        let ratio = effective_price_a / effective_price_b;

        Ok(VolumeBasedRatio {
            pair_name: pair_name.to_string(),
            symbol_a: symbol_a.to_string(),
            symbol_b: symbol_b.to_string(),
            volume,
            effective_price_a,
            effective_price_b,
            ratio,
            slippage_a,
            slippage_b,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Analyze slippage for a specific trade volume
    pub async fn analyze_slippage(
        &self,
        symbol: &str,
        volume: f64,
        side: OrderSide,
    ) -> Result<SlippageAnalysis> {
        let order_book = self.client.get_order_book(symbol, 100).await?;

        let mid_price = (order_book.best_bid + order_book.best_ask) / 2.0;
        let (effective_price, slippage_pct, depth_consumed, total_cost) =
            match side {
                OrderSide::Buy => {
                    let (eff_price, slippage) = Self::calculate_effective_price(&order_book, volume, OrderSide::Buy)?;
                    let depth = Self::calculate_depth_consumed(&order_book.asks, volume);
                    let cost = eff_price * volume;
                    (eff_price, slippage, depth, cost)
                }
                OrderSide::Sell => {
                    let (eff_price, slippage) = Self::calculate_effective_price(&order_book, volume, OrderSide::Sell)?;
                    let depth = Self::calculate_depth_consumed(&order_book.bids, volume);
                    let cost = eff_price * volume;
                    (eff_price, slippage, depth, cost)
                }
            };

        Ok(SlippageAnalysis {
            symbol: symbol.to_string(),
            mid_price,
            volume,
            side,
            effective_price,
            slippage_percentage: slippage_pct,
            depth_consumed,
            total_cost,
        })
    }

    /// Calculate effective price considering order book depth and slippage
    fn calculate_effective_price(
        order_book: &OrderBookInfo,
        volume: f64,
        side: OrderSide,
    ) -> Result<(f64, f64)> {
        let (levels, best_price) = match side {
            OrderSide::Buy => (&order_book.asks, order_book.best_ask),
            OrderSide::Sell => (&order_book.bids, order_book.best_bid),
        };

        let mut remaining_volume = volume;
        let mut total_cost = 0.0;
        let mut filled_volume = 0.0;

        for (price, quantity) in levels {
            if remaining_volume <= 0.0 {
                break;
            }

            let fill_qty = remaining_volume.min(*quantity);
            total_cost += fill_qty * price;
            filled_volume += fill_qty;
            remaining_volume -= fill_qty;
        }

        if filled_volume < volume {
            anyhow::bail!(
                "Insufficient liquidity in order book for {} {}. Requested: {}, Available: {}",
                order_book.symbol,
                match side { OrderSide::Buy => "asks", OrderSide::Sell => "bids" },
                volume,
                filled_volume
            );
        }

        let effective_price = total_cost / filled_volume;
        let slippage_percentage = ((effective_price - best_price) / best_price).abs() * 100.0;

        Ok((effective_price, slippage_percentage))
    }

    /// Calculate how many order book levels were consumed
    fn calculate_depth_consumed(levels: &[(f64, f64)], volume: f64) -> usize {
        let mut remaining = volume;
        let mut count = 0;

        for (_, quantity) in levels {
            if remaining <= 0.0 {
                break;
            }
            remaining -= quantity;
            count += 1;
        }

        count
    }
}

impl SimpleRatio {
    pub fn format_summary(&self) -> String {
        format!(
            "{}: {:.8} ({}=${:.2} / {}=${:.2})",
            self.pair_name,
            self.ratio,
            self.symbol_a,
            self.price_a,
            self.symbol_b,
            self.price_b
        )
    }
}

impl VolumeBasedRatio {
    pub fn format_summary(&self) -> String {
        format!(
            "{}: {:.8} [Vol: {}]\n  {} eff=${:.2} (slippage: {:.3}%)\n  {} eff=${:.2} (slippage: {:.3}%)",
            self.pair_name,
            self.ratio,
            self.volume,
            self.symbol_a,
            self.effective_price_a,
            self.slippage_a,
            self.symbol_b,
            self.effective_price_b,
            self.slippage_b
        )
    }
}

impl SlippageAnalysis {
    pub fn format_summary(&self) -> String {
        format!(
            "{} {:?} {:.4} units:\n  Mid: ${:.2} → Effective: ${:.2}\n  Slippage: {:.3}%\n  Depth consumed: {} levels\n  Total cost: ${:.2}",
            self.symbol,
            self.side,
            self.volume,
            self.mid_price,
            self.effective_price,
            self.slippage_percentage,
            self.depth_consumed,
            self.total_cost
        )
    }
}
