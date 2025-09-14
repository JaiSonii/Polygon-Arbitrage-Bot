use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub token0: String,
    pub token1: String,
    pub token0_symbol: String,
    pub token1_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    pub dex_name: String,
    pub token_pair: TokenPair,
    pub price: BigDecimal,
    pub timestamp: DateTime<Utc>,
    pub liquidity: Option<BigDecimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: Uuid,
    pub token_pair: TokenPair,
    pub buy_dex: String,
    pub sell_dex: String,
    pub buy_price: BigDecimal,
    pub sell_price: BigDecimal,
    pub price_difference: BigDecimal,
    pub price_difference_percentage: BigDecimal,
    pub estimated_profit: BigDecimal,
    pub trade_amount: BigDecimal,
    pub gas_cost: BigDecimal,
    pub net_profit: BigDecimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DexPrices {
    pub dex_name: String,
    pub quotes: Vec<PriceQuote>,
}

impl ArbitrageOpportunity {
    pub fn new(
        token_pair: TokenPair,
        buy_dex: String,
        sell_dex: String,
        buy_price: BigDecimal,
        sell_price: BigDecimal,
        trade_amount: BigDecimal,
        gas_cost: BigDecimal,
    ) -> Self {
        let price_difference = &sell_price - &buy_price;
        let price_difference_percentage = if buy_price > BigDecimal::from(0) {
            (&price_difference / &buy_price) * BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };
        
        let estimated_profit = &price_difference * &trade_amount;
        let net_profit = &estimated_profit - &gas_cost;

        Self {
            id: Uuid::new_v4(),
            token_pair,
            buy_dex,
            sell_dex,
            buy_price,
            sell_price,
            price_difference,
            price_difference_percentage,
            estimated_profit,
            trade_amount,
            gas_cost,
            net_profit,
            timestamp: Utc::now(),
        }
    }
}
