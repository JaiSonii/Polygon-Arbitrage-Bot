use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArbitrageOpportunityRow {
    pub id: Uuid,
    pub token0_address: String,
    pub token1_address: String,
    pub token0_symbol: String,
    pub token1_symbol: String,
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
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PriceQuoteRow {
    pub id: Uuid,
    pub dex_name: String,
    pub token0_address: String,
    pub token1_address: String,
    pub token0_symbol: String,
    pub token1_symbol: String,
    pub price: BigDecimal,
    pub liquidity: Option<BigDecimal>,
    pub timestamp: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityStats {
    pub total_opportunities: i64,
    pub total_profit: BigDecimal,
    pub average_profit: BigDecimal,
    pub best_opportunity_profit: BigDecimal,
    pub most_active_dex_pair: Option<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexStats {
    pub dex_name: String,
    pub total_quotes: i64,
    pub average_price: BigDecimal,
    pub price_volatility: BigDecimal,
    pub last_update: DateTime<Utc>,
}

impl From<crate::types::ArbitrageOpportunity> for ArbitrageOpportunityRow {
    fn from(opportunity: crate::types::ArbitrageOpportunity) -> Self {
        Self {
            id: opportunity.id,
            token0_address: opportunity.token_pair.token0,
            token1_address: opportunity.token_pair.token1,
            token0_symbol: opportunity.token_pair.token0_symbol,
            token1_symbol: opportunity.token_pair.token1_symbol,
            buy_dex: opportunity.buy_dex,
            sell_dex: opportunity.sell_dex,
            buy_price: opportunity.buy_price,
            sell_price: opportunity.sell_price,
            price_difference: opportunity.price_difference,
            price_difference_percentage: opportunity.price_difference_percentage,
            estimated_profit: opportunity.estimated_profit,
            trade_amount: opportunity.trade_amount,
            gas_cost: opportunity.gas_cost,
            net_profit: opportunity.net_profit,
            timestamp: opportunity.timestamp,
            created_at: None,
        }
    }
}

impl From<ArbitrageOpportunityRow> for crate::types::ArbitrageOpportunity {
    fn from(row: ArbitrageOpportunityRow) -> Self {
        Self {
            id: row.id,
            token_pair: crate::types::TokenPair {
                token0: row.token0_address,
                token1: row.token1_address,
                token0_symbol: row.token0_symbol,
                token1_symbol: row.token1_symbol,
            },
            buy_dex: row.buy_dex,
            sell_dex: row.sell_dex,
            buy_price: row.buy_price,
            sell_price: row.sell_price,
            price_difference: row.price_difference,
            price_difference_percentage: row.price_difference_percentage,
            estimated_profit: row.estimated_profit,
            trade_amount: row.trade_amount,
            gas_cost: row.gas_cost,
            net_profit: row.net_profit,
            timestamp: row.timestamp,
        }
    }
}

impl From<crate::types::PriceQuote> for PriceQuoteRow {
    fn from(quote: crate::types::PriceQuote) -> Self {
        Self {
            id: Uuid::new_v4(),
            dex_name: quote.dex_name,
            token0_address: quote.token_pair.token0,
            token1_address: quote.token_pair.token1,
            token0_symbol: quote.token_pair.token0_symbol,
            token1_symbol: quote.token_pair.token1_symbol,
            price: quote.price,
            liquidity: quote.liquidity,
            timestamp: quote.timestamp,
            created_at: None,
        }
    }
}

impl From<PriceQuoteRow> for crate::types::PriceQuote {
    fn from(row: PriceQuoteRow) -> Self {
        Self {
            dex_name: row.dex_name,
            token_pair: crate::types::TokenPair {
                token0: row.token0_address,
                token1: row.token1_address,
                token0_symbol: row.token0_symbol,
                token1_symbol: row.token1_symbol,
            },
            price: row.price,
            timestamp: row.timestamp,
            liquidity: row.liquidity,
        }
    }
}
