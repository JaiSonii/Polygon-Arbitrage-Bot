use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use std::str::FromStr;
use tracing::{debug, info, warn};

use crate::{
    config::ArbitrageConfig,
    types::{ArbitrageOpportunity, PriceQuote, TokenPair},
};

pub struct ArbitrageDetector {
    config: ArbitrageConfig,
    min_profit_threshold: BigDecimal,
    trade_amount: BigDecimal,
    gas_cost_estimate: BigDecimal,
}

impl ArbitrageDetector {
    pub fn new(config: ArbitrageConfig) -> Result<Self> {
        let min_profit_threshold = BigDecimal::from_str(&config.min_profit_threshold)
            .map_err(|e| anyhow!("Invalid min_profit_threshold: {}", e))?;
        
        let trade_amount = BigDecimal::from_str(&config.trade_amount)
            .map_err(|e| anyhow!("Invalid trade_amount: {}", e))?;
        
        let gas_cost_estimate = BigDecimal::from_str(&config.gas_cost_estimate)
            .map_err(|e| anyhow!("Invalid gas_cost_estimate: {}", e))?;

        Ok(Self {
            config,
            min_profit_threshold,
            trade_amount,
            gas_cost_estimate,
        })
    }

    pub fn detect_opportunities(&self, quotes: &[PriceQuote]) -> Result<Vec<ArbitrageOpportunity>> {
        if quotes.len() < 2 {
            debug!("Not enough quotes to detect arbitrage opportunities");
            return Ok(Vec::new());
        }

        let mut opportunities = Vec::new();

        // Compare all pairs of quotes to find arbitrage opportunities
        for i in 0..quotes.len() {
            for j in (i + 1)..quotes.len() {
                let quote1 = &quotes[i];
                let quote2 = &quotes[j];

                // Check both directions: buy from quote1, sell to quote2 and vice versa
                if let Some(opportunity) = self.analyze_quote_pair(quote1, quote2)? {
                    opportunities.push(opportunity);
                }
                
                if let Some(opportunity) = self.analyze_quote_pair(quote2, quote1)? {
                    opportunities.push(opportunity);
                }
            }
        }

        // Filter opportunities by minimum profit threshold
        let profitable_opportunities: Vec<ArbitrageOpportunity> = opportunities
            .into_iter()
            .filter(|opp| opp.net_profit >= self.min_profit_threshold)
            .collect();

        if !profitable_opportunities.is_empty() {
            info!(
                "Found {} profitable arbitrage opportunities",
                profitable_opportunities.len()
            );
        }

        Ok(profitable_opportunities)
    }

    fn analyze_quote_pair(
        &self,
        buy_quote: &PriceQuote,
        sell_quote: &PriceQuote,
    ) -> Result<Option<ArbitrageOpportunity>> {
        // Ensure we're comparing the same token pair
        if !self.is_same_token_pair(&buy_quote.token_pair, &sell_quote.token_pair) {
            return Ok(None);
        }

        // Skip if prices are the same (no arbitrage opportunity)
        if buy_quote.price == sell_quote.price {
            return Ok(None);
        }

        // Check if there's a profitable arbitrage opportunity
        // We want to buy low and sell high
        if sell_quote.price <= buy_quote.price {
            return Ok(None);
        }

        let opportunity = ArbitrageOpportunity::new(
            buy_quote.token_pair.clone(),
            buy_quote.dex_name.clone(),
            sell_quote.dex_name.clone(),
            buy_quote.price.clone(),
            sell_quote.price.clone(),
            self.trade_amount.clone(),
            self.gas_cost_estimate.clone(),
        );

        // Additional validation
        if opportunity.net_profit <= BigDecimal::from(0) {
            debug!(
                "Opportunity between {} and {} has negative net profit: {}",
                buy_quote.dex_name, sell_quote.dex_name, opportunity.net_profit
            );
            return Ok(None);
        }

        debug!(
            "Potential arbitrage: Buy {} at {} for {}, sell at {} for {}, net profit: {}",
            opportunity.token_pair.token0_symbol,
            opportunity.buy_dex,
            opportunity.buy_price,
            opportunity.sell_dex,
            opportunity.sell_price,
            opportunity.net_profit
        );

        Ok(Some(opportunity))
    }

    fn is_same_token_pair(&self, pair1: &TokenPair, pair2: &TokenPair) -> bool {
        (pair1.token0 == pair2.token0 && pair1.token1 == pair2.token1) ||
        (pair1.token0 == pair2.token1 && pair1.token1 == pair2.token0)
    }

    pub fn get_min_profit_threshold(&self) -> &BigDecimal {
        &self.min_profit_threshold
    }

    pub fn get_trade_amount(&self) -> &BigDecimal {
        &self.trade_amount
    }

    pub fn get_gas_cost_estimate(&self) -> &BigDecimal {
        &self.gas_cost_estimate
    }

    pub fn update_gas_cost_estimate(&mut self, new_gas_cost: BigDecimal) {
        self.gas_cost_estimate = new_gas_cost;
        info!("Updated gas cost estimate to: {}", self.gas_cost_estimate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_config() -> ArbitrageConfig {
        ArbitrageConfig {
            min_profit_threshold: "5.0".to_string(),
            trade_amount: "1000.0".to_string(),
            gas_cost_estimate: "2.0".to_string(),
            check_interval_seconds: 30,
        }
    }

    fn create_test_token_pair() -> TokenPair {
        TokenPair {
            token0: "0x123".to_string(),
            token1: "0x456".to_string(),
            token0_symbol: "WETH".to_string(),
            token1_symbol: "USDC".to_string(),
        }
    }

    fn create_test_quote(dex_name: &str, price: f64) -> PriceQuote {
        PriceQuote {
            dex_name: dex_name.to_string(),
            token_pair: create_test_token_pair(),
            price: BigDecimal::from(price),
            timestamp: Utc::now(),
            liquidity: None,
        }
    }

    #[test]
    fn test_detect_opportunities() {
        let config = create_test_config();
        let detector = ArbitrageDetector::new(config).unwrap();

        let quotes = vec![
            create_test_quote("Uniswap", 2000.0),
            create_test_quote("QuickSwap", 2010.0),
        ];

        let opportunities = detector.detect_opportunities(&quotes).unwrap();
        assert_eq!(opportunities.len(), 1);

        let opp = &opportunities[0];
        assert_eq!(opp.buy_dex, "Uniswap");
        assert_eq!(opp.sell_dex, "QuickSwap");
        assert_eq!(opp.buy_price, BigDecimal::from(2000.0));
        assert_eq!(opp.sell_price, BigDecimal::from(2010.0));
    }

    #[test]
    fn test_no_opportunities_same_price() {
        let config = create_test_config();
        let detector = ArbitrageDetector::new(config).unwrap();

        let quotes = vec![
            create_test_quote("Uniswap", 2000.0),
            create_test_quote("QuickSwap", 2000.0),
        ];

        let opportunities = detector.detect_opportunities(&quotes).unwrap();
        assert_eq!(opportunities.len(), 0);
    }

    #[test]
    fn test_filter_by_min_profit() {
        let mut config = create_test_config();
        config.min_profit_threshold = "20.0".to_string(); // High threshold
        let detector = ArbitrageDetector::new(config).unwrap();

        let quotes = vec![
            create_test_quote("Uniswap", 2000.0),
            create_test_quote("QuickSwap", 2005.0), // Small difference
        ];

        let opportunities = detector.detect_opportunities(&quotes).unwrap();
        assert_eq!(opportunities.len(), 0); // Should be filtered out
    }
}
