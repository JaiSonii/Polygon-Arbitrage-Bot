use anyhow::Result;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::types::{PriceQuote, TokenPair};

pub struct PriceAggregator {
    price_cache: HashMap<String, Vec<PriceQuote>>,
    cache_duration_seconds: u64,
}

impl PriceAggregator {
    pub fn new(cache_duration_seconds: u64) -> Self {
        Self {
            price_cache: HashMap::new(),
            cache_duration_seconds,
        }
    }

    pub fn cache_prices(&mut self, token_pair: &TokenPair, quotes: Vec<PriceQuote>) {
        let cache_key = self.generate_cache_key(token_pair);
        self.price_cache.insert(cache_key, quotes);
    }

    pub fn get_cached_prices(&self, token_pair: &TokenPair) -> Option<&Vec<PriceQuote>> {
        let cache_key = self.generate_cache_key(token_pair);
        
        if let Some(quotes) = self.price_cache.get(&cache_key) {
            // Check if cache is still valid
            if let Some(first_quote) = quotes.first() {
                let now = chrono::Utc::now();
                let cache_age = now.signed_duration_since(first_quote.timestamp);
                
                if cache_age.num_seconds() < self.cache_duration_seconds as i64 {
                    return Some(quotes);
                }
            }
        }
        
        None
    }

    pub fn find_best_prices(&self, quotes: &[PriceQuote]) -> (Option<&PriceQuote>, Option<&PriceQuote>) {
        if quotes.is_empty() {
            return (None, None);
        }

        let mut lowest_price: Option<&PriceQuote> = None;
        let mut highest_price: Option<&PriceQuote> = None;

        for quote in quotes {
            match &lowest_price {
                None => lowest_price = Some(quote),
                Some(current_lowest) => {
                    if quote.price < current_lowest.price {
                        lowest_price = Some(quote);
                    }
                }
            }

            match &highest_price {
                None => highest_price = Some(quote),
                Some(current_highest) => {
                    if quote.price > current_highest.price {
                        highest_price = Some(quote);
                    }
                }
            }
        }

        (lowest_price, highest_price)
    }

    pub fn calculate_price_spread(&self, quotes: &[PriceQuote]) -> Option<BigDecimal> {
        let (lowest, highest) = self.find_best_prices(quotes);
        
        match (lowest, highest) {
            (Some(low), Some(high)) => {
                if low.price > BigDecimal::from(0) {
                    let spread = (&high.price - &low.price) / &low.price * BigDecimal::from(100);
                    Some(spread)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn filter_valid_quotes(&self, quotes: Vec<PriceQuote>) -> Vec<PriceQuote> {
        quotes
            .into_iter()
            .filter(|quote| {
                // Filter out quotes with zero or negative prices
                if quote.price <= BigDecimal::from(0) {
                    warn!("Filtering out invalid quote with price: {}", quote.price);
                    return false;
                }
                
                // Filter out quotes that are too old
                let now = chrono::Utc::now();
                let quote_age = now.signed_duration_since(quote.timestamp);
                if quote_age.num_seconds() > self.cache_duration_seconds as i64 * 2 {
                    warn!("Filtering out stale quote from {}", quote.dex_name);
                    return false;
                }
                
                true
            })
            .collect()
    }

    fn generate_cache_key(&self, token_pair: &TokenPair) -> String {
        format!("{}_{}", token_pair.token0, token_pair.token1)
    }

    pub fn clear_cache(&mut self) {
        self.price_cache.clear();
        debug!("Price cache cleared");
    }

    pub fn cache_size(&self) -> usize {
        self.price_cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_quote(dex_name: &str, price: f64) -> PriceQuote {
        PriceQuote {
            dex_name: dex_name.to_string(),
            token_pair: TokenPair {
                token0: "0x123".to_string(),
                token1: "0x456".to_string(),
                token0_symbol: "TOKEN0".to_string(),
                token1_symbol: "TOKEN1".to_string(),
            },
            price: BigDecimal::from(price),
            timestamp: Utc::now(),
            liquidity: None,
        }
    }

    #[test]
    fn test_find_best_prices() {
        let aggregator = PriceAggregator::new(60);
        let quotes = vec![
            create_test_quote("DEX1", 100.0),
            create_test_quote("DEX2", 95.0),
            create_test_quote("DEX3", 105.0),
        ];

        let (lowest, highest) = aggregator.find_best_prices(&quotes);
        
        assert!(lowest.is_some());
        assert!(highest.is_some());
        assert_eq!(lowest.unwrap().price, BigDecimal::from(95.0));
        assert_eq!(highest.unwrap().price, BigDecimal::from(105.0));
    }

    #[test]
    fn test_calculate_price_spread() {
        let aggregator = PriceAggregator::new(60);
        let quotes = vec![
            create_test_quote("DEX1", 100.0),
            create_test_quote("DEX2", 110.0),
        ];

        let spread = aggregator.calculate_price_spread(&quotes);
        assert!(spread.is_some());
        // Spread should be 10% ((110-100)/100 * 100)
        assert_eq!(spread.unwrap(), BigDecimal::from(10));
    }
}
