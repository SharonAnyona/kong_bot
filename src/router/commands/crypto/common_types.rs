use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};

// Coingecko API response structures
#[derive(Debug, Deserialize)]
pub struct PriceResponse {
    #[serde(flatten)]
    pub prices: std::collections::HashMap<String, CurrencyPrices>,
    #[serde(skip)]
    pub mapped_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CurrencyPrices {
    #[serde(flatten)]
    pub values: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
pub struct TrendingResponse {
    pub coins: Vec<TrendingCoin>,
}

#[derive(Debug, Deserialize)]
pub struct TrendingCoin {
    pub item: TrendingCoinItem,
}

#[derive(Debug, Deserialize)]
pub struct TrendingCoinItem {
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub market_cap_rank: Option<u32>,
    pub price_btc: f64,
}

#[derive(Debug, Deserialize)]
pub struct MarketData {
    #[allow(dead_code)]
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub current_price: std::collections::HashMap<String, f64>,
    #[allow(dead_code)]
    pub market_cap: std::collections::HashMap<String, f64>,
    #[allow(dead_code)]
    pub market_cap_rank: Option<u32>,
    pub price_change_percentage_24h: Option<f64>,
}

// Result type for trending coin fetching
pub struct TrendingResult {
    pub trending_data: TrendingResponse,
    pub prices_data:
        Option<std::collections::HashMap<String, std::collections::HashMap<String, f64>>>,
}

// Alert types
#[derive(Debug, Clone)]
pub struct PriceAlert {
    pub user_id: String,
    pub coin_id: String,
    pub target_price: f64,
    pub currency: String,
    pub is_above: bool, // true if alert when price goes above target
}

#[derive(Debug, Default)]
pub struct AlertStorage {
    pub alerts: HashMap<String, Vec<PriceAlert>>, // coin_id -> alerts
}

// Global alert storage
pub static ALERTS: once_cell::sync::Lazy<Arc<Mutex<AlertStorage>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(AlertStorage::default())));

#[derive(Debug, Clone)]
pub struct TradingRule {
    pub user_id: String,
    pub wallet_address: String,
    pub coin_id: String,
    pub currency: String,
    pub buy_threshold: f64,
    pub sell_threshold: f64,
}

// Global storage for trading rules
pub static TRADING_RULES: once_cell::sync::Lazy<std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, TradingRule>>>> =
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())));



#[derive(Debug, Clone)]
pub struct TradeOrder {
    pub user_id: String,
    pub coin_id: String,
    pub target_price: f64,
    pub amount: f64,
    pub wallet: String,
    pub is_buy: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}   