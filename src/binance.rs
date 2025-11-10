use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use reqwest::Client;

const BINANCE_API_BASE: &str = "https://api.binance.com/api/v3";

#[derive(Debug, Clone)]
pub struct BinanceClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct TickerPrice {
    pub symbol: String,
    pub price: String,
}

#[derive(Debug, Deserialize)]
pub struct OrderBook {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    pub bids: Vec<(String, String)>, // price, quantity
    pub asks: Vec<(String, String)>, // price, quantity
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceInfo {
    pub symbol: String,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrderBookInfo {
    pub symbol: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub bids: Vec<(f64, f64)>, // price, quantity
    pub asks: Vec<(f64, f64)>, // price, quantity
}

impl BinanceClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Fetch current price for a symbol
    pub async fn get_price(&self, symbol: &str) -> Result<PriceInfo> {
        let url = format!("{}/ticker/price?symbol={}", BINANCE_API_BASE, symbol);

        let response = self.client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch price for {}", symbol))?;

        let ticker: TickerPrice = response
            .json()
            .await
            .with_context(|| format!("Failed to parse price response for {}", symbol))?;

        let price = ticker.price.parse::<f64>()
            .with_context(|| format!("Failed to parse price value: {}", ticker.price))?;

        Ok(PriceInfo {
            symbol: ticker.symbol,
            price,
        })
    }

    /// Fetch order book for a symbol
    pub async fn get_order_book(&self, symbol: &str, limit: u32) -> Result<OrderBookInfo> {
        let url = format!(
            "{}/depth?symbol={}&limit={}",
            BINANCE_API_BASE, symbol, limit
        );

        let response = self.client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch order book for {}", symbol))?;

        let order_book: OrderBook = response
            .json()
            .await
            .with_context(|| format!("Failed to parse order book response for {}", symbol))?;

        // Parse bids and asks
        let bids: Result<Vec<(f64, f64)>> = order_book.bids
            .iter()
            .map(|(price, qty)| {
                let p = price.parse::<f64>()?;
                let q = qty.parse::<f64>()?;
                Ok((p, q))
            })
            .collect();

        let asks: Result<Vec<(f64, f64)>> = order_book.asks
            .iter()
            .map(|(price, qty)| {
                let p = price.parse::<f64>()?;
                let q = qty.parse::<f64>()?;
                Ok((p, q))
            })
            .collect();

        let bids = bids?;
        let asks = asks?;

        let best_bid = bids.first().map(|(p, _)| *p).unwrap_or(0.0);
        let best_ask = asks.first().map(|(p, _)| *p).unwrap_or(0.0);

        Ok(OrderBookInfo {
            symbol: symbol.to_string(),
            best_bid,
            best_ask,
            bids,
            asks,
        })
    }

    /// Fetch prices for multiple symbols in parallel
    pub async fn get_prices(&self, symbols: &[String]) -> Result<Vec<PriceInfo>> {
        let mut tasks = Vec::new();

        for symbol in symbols {
            let client = self.clone();
            let symbol = symbol.clone();
            tasks.push(tokio::spawn(async move {
                client.get_price(&symbol).await
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            let result = task.await??;
            results.push(result);
        }

        Ok(results)
    }

    /// Fetch order books for multiple symbols in parallel
    pub async fn get_order_books(&self, symbols: &[String], limit: u32) -> Result<Vec<OrderBookInfo>> {
        let mut tasks = Vec::new();

        for symbol in symbols {
            let client = self.clone();
            let symbol = symbol.clone();
            tasks.push(tokio::spawn(async move {
                client.get_order_book(&symbol, limit).await
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            let result = task.await??;
            results.push(result);
        }

        Ok(results)
    }
}

impl Default for BinanceClient {
    fn default() -> Self {
        Self::new()
    }
}
