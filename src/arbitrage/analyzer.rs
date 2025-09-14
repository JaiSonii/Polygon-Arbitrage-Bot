use anyhow::Result;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::types::{ArbitrageOpportunity, PriceQuote};

pub struct OpportunityAnalyzer {
    historical_opportunities: Vec<ArbitrageOpportunity>,
    dex_performance: HashMap<String, DexPerformanceMetrics>,
}

#[derive(Debug, Clone)]
pub struct DexPerformanceMetrics {
    pub total_opportunities: u64,
    pub average_profit: BigDecimal,
    pub success_rate: f64,
    pub average_execution_time: u64,
}

#[derive(Debug, Clone)]
pub struct MarketAnalysis {
    pub total_opportunities_found: u64,
    pub average_profit_per_opportunity: BigDecimal,
    pub most_profitable_pair: Option<String>,
    pub best_performing_dex_pair: Option<(String, String)>,
    pub market_efficiency_score: f64,
}

impl OpportunityAnalyzer {
    pub fn new() -> Self {
        Self {
            historical_opportunities: Vec::new(),
            dex_performance: HashMap::new(),
        }
    }

    pub fn add_opportunity(&mut self, opportunity: ArbitrageOpportunity) {
        // Update DEX performance metrics
        self.update_dex_metrics(&opportunity);
        
        // Store the opportunity
        self.historical_opportunities.push(opportunity);
        
        // Keep only recent opportunities (last 1000)
        if self.historical_opportunities.len() > 1000 {
            self.historical_opportunities.remove(0);
        }
    }

    pub fn analyze_market_efficiency(&self, quotes: &[PriceQuote]) -> f64 {
        if quotes.len() < 2 {
            return 1.0; // Perfect efficiency if only one price source
        }

        let mut price_deviations = Vec::new();
        let average_price = self.calculate_average_price(quotes);

        for quote in quotes {
            let deviation = if average_price > BigDecimal::from(0) {
                ((&quote.price - &average_price) / &average_price).abs()
            } else {
                BigDecimal::from(0)
            };
            
            if let Ok(deviation_f64) = deviation.to_string().parse::<f64>() {
                price_deviations.push(deviation_f64);
            }
        }

        if price_deviations.is_empty() {
            return 1.0;
        }

        let average_deviation: f64 = price_deviations.iter().sum::<f64>() / price_deviations.len() as f64;
        
        // Efficiency score: 1.0 - average_deviation (capped at 0.0)
        (1.0 - average_deviation).max(0.0)
    }

    pub fn generate_market_analysis(&self) -> MarketAnalysis {
        let total_opportunities = self.historical_opportunities.len() as u64;
        
        let average_profit = if total_opportunities > 0 {
            let total_profit: BigDecimal = self.historical_opportunities
                .iter()
                .map(|opp| &opp.net_profit)
                .sum();
            total_profit / BigDecimal::from(total_opportunities)
        } else {
            BigDecimal::from(0)
        };

        let most_profitable_pair = self.find_most_profitable_token_pair();
        let best_performing_dex_pair = self.find_best_dex_pair();
        
        // Calculate market efficiency based on recent opportunities
        let recent_opportunities: Vec<&ArbitrageOpportunity> = self.historical_opportunities
            .iter()
            .rev()
            .take(100)
            .collect();
        
        let market_efficiency_score = if recent_opportunities.len() > 10 {
            // More opportunities indicate less efficient market
            let opportunity_frequency = recent_opportunities.len() as f64 / 100.0;
            (1.0 - opportunity_frequency).max(0.0)
        } else {
            0.9 // Assume high efficiency if few opportunities
        };

        MarketAnalysis {
            total_opportunities_found: total_opportunities,
            average_profit_per_opportunity: average_profit,
            most_profitable_pair,
            best_performing_dex_pair,
            market_efficiency_score,
        }
    }

    pub fn get_dex_performance(&self, dex_name: &str) -> Option<&DexPerformanceMetrics> {
        self.dex_performance.get(dex_name)
    }

    pub fn recommend_optimal_trade_size(&self, token_pair: &str) -> BigDecimal {
        let relevant_opportunities: Vec<&ArbitrageOpportunity> = self.historical_opportunities
            .iter()
            .filter(|opp| {
                format!("{}/{}", opp.token_pair.token0_symbol, opp.token_pair.token1_symbol) == token_pair
            })
            .collect();

        if relevant_opportunities.is_empty() {
            return BigDecimal::from(1000); // Default trade size
        }

        // Find the trade size that historically yielded the best ROI
        let mut best_roi = BigDecimal::from(0);
        let mut optimal_size = BigDecimal::from(1000);

        for opportunity in relevant_opportunities {
            let investment = &opportunity.trade_amount * &opportunity.buy_price;
            if investment > BigDecimal::from(0) {
                let roi = &opportunity.net_profit / investment;
                if roi > best_roi {
                    best_roi = roi;
                    optimal_size = opportunity.trade_amount.clone();
                }
            }
        }

        optimal_size
    }

    fn update_dex_metrics(&mut self, opportunity: &ArbitrageOpportunity) {
        // Update metrics for buy DEX
        let buy_metrics = self.dex_performance
            .entry(opportunity.buy_dex.clone())
            .or_insert_with(|| DexPerformanceMetrics {
                total_opportunities: 0,
                average_profit: BigDecimal::from(0),
                success_rate: 0.0,
                average_execution_time: 30,
            });
        
        buy_metrics.total_opportunities += 1;
        buy_metrics.average_profit = (&buy_metrics.average_profit + &opportunity.net_profit) / BigDecimal::from(2);

        // Update metrics for sell DEX
        let sell_metrics = self.dex_performance
            .entry(opportunity.sell_dex.clone())
            .or_insert_with(|| DexPerformanceMetrics {
                total_opportunities: 0,
                average_profit: BigDecimal::from(0),
                success_rate: 0.0,
                average_execution_time: 30,
            });
        
        sell_metrics.total_opportunities += 1;
        sell_metrics.average_profit = (&sell_metrics.average_profit + &opportunity.net_profit) / BigDecimal::from(2);
    }

    fn calculate_average_price(&self, quotes: &[PriceQuote]) -> BigDecimal {
        if quotes.is_empty() {
            return BigDecimal::from(0);
        }

        let total: BigDecimal = quotes.iter().map(|q| &q.price).sum();
        total / BigDecimal::from(quotes.len())
    }

    fn find_most_profitable_token_pair(&self) -> Option<String> {
        let mut pair_profits: HashMap<String, BigDecimal> = HashMap::new();

        for opportunity in &self.historical_opportunities {
            let pair_key = format!("{}/{}", 
                opportunity.token_pair.token0_symbol, 
                opportunity.token_pair.token1_symbol
            );
            
            let current_profit = pair_profits.get(&pair_key).cloned().unwrap_or_else(|| BigDecimal::from(0));
            pair_profits.insert(pair_key, current_profit + &opportunity.net_profit);
        }

        pair_profits
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(pair, _)| pair)
    }

    fn find_best_dex_pair(&self) -> Option<(String, String)> {
        let mut dex_pair_profits: HashMap<(String, String), BigDecimal> = HashMap::new();

        for opportunity in &self.historical_opportunities {
            let pair_key = (opportunity.buy_dex.clone(), opportunity.sell_dex.clone());
            let current_profit = dex_pair_profits.get(&pair_key).cloned().unwrap_or_else(|| BigDecimal::from(0));
            dex_pair_profits.insert(pair_key, current_profit + &opportunity.net_profit);
        }

        dex_pair_profits
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(pair, _)| pair)
    }

    pub fn clear_history(&mut self) {
        self.historical_opportunities.clear();
        self.dex_performance.clear();
        info!("Cleared opportunity analysis history");
    }

    pub fn get_opportunity_count(&self) -> usize {
        self.historical_opportunities.len()
    }
}

impl Default for OpportunityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
