use anyhow::Result;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotMetrics {
    pub uptime_seconds: u64,
    pub total_cycles_completed: u64,
    pub total_opportunities_found: u64,
    pub total_profit_simulated: BigDecimal,
    pub average_profit_per_opportunity: BigDecimal,
    pub success_rate: f64,
    pub dex_performance: HashMap<String, DexMetrics>,
    pub token_pair_performance: HashMap<String, TokenPairMetrics>,
    pub error_count: u64,
    pub last_error: Option<String>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexMetrics {
    pub name: String,
    pub total_quotes_fetched: u64,
    pub successful_quotes: u64,
    pub failed_quotes: u64,
    pub average_response_time_ms: f64,
    pub opportunities_as_buy_side: u64,
    pub opportunities_as_sell_side: u64,
    pub total_profit_contribution: BigDecimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPairMetrics {
    pub pair: String,
    pub total_opportunities: u64,
    pub total_profit: BigDecimal,
    pub average_profit: BigDecimal,
    pub best_profit: BigDecimal,
    pub average_price_spread: f64,
    pub market_efficiency_score: f64,
}

impl BotMetrics {
    pub fn new() -> Self {
        Self {
            uptime_seconds: 0,
            total_cycles_completed: 0,
            total_opportunities_found: 0,
            total_profit_simulated: BigDecimal::from(0),
            average_profit_per_opportunity: BigDecimal::from(0),
            success_rate: 0.0,
            dex_performance: HashMap::new(),
            token_pair_performance: HashMap::new(),
            error_count: 0,
            last_error: None,
            last_updated: Utc::now(),
        }
    }

    pub fn update_cycle_metrics(&mut self, opportunities_found: u64, cycle_profit: BigDecimal) {
        self.total_cycles_completed += 1;
        self.total_opportunities_found += opportunities_found;
        self.total_profit_simulated += cycle_profit;
        
        if self.total_opportunities_found > 0 {
            self.average_profit_per_opportunity = 
                &self.total_profit_simulated / BigDecimal::from(self.total_opportunities_found);
        }
        
        self.last_updated = Utc::now();
    }

    pub fn update_dex_metrics(&mut self, dex_name: &str, success: bool, response_time_ms: f64) {
        let metrics = self.dex_performance.entry(dex_name.to_string())
            .or_insert_with(|| DexMetrics {
                name: dex_name.to_string(),
                total_quotes_fetched: 0,
                successful_quotes: 0,
                failed_quotes: 0,
                average_response_time_ms: 0.0,
                opportunities_as_buy_side: 0,
                opportunities_as_sell_side: 0,
                total_profit_contribution: BigDecimal::from(0),
            });

        metrics.total_quotes_fetched += 1;
        
        if success {
            metrics.successful_quotes += 1;
        } else {
            metrics.failed_quotes += 1;
        }

        // Update average response time
        let total_time = metrics.average_response_time_ms * (metrics.total_quotes_fetched - 1) as f64;
        metrics.average_response_time_ms = (total_time + response_time_ms) / metrics.total_quotes_fetched as f64;
    }

    pub fn record_error(&mut self, error_message: &str) {
        self.error_count += 1;
        self.last_error = Some(error_message.to_string());
        self.last_updated = Utc::now();
    }

    pub fn calculate_success_rate(&mut self) {
        if self.total_cycles_completed > 0 {
            let successful_cycles = self.total_cycles_completed - self.error_count;
            self.success_rate = successful_cycles as f64 / self.total_cycles_completed as f64;
        }
    }

    pub fn update_token_pair_metrics(&mut self, pair: &str, profit: BigDecimal, price_spread: f64) {
        let metrics = self.token_pair_performance.entry(pair.to_string())
            .or_insert_with(|| TokenPairMetrics {
                pair: pair.to_string(),
                total_opportunities: 0,
                total_profit: BigDecimal::from(0),
                average_profit: BigDecimal::from(0),
                best_profit: BigDecimal::from(0),
                average_price_spread: 0.0,
                market_efficiency_score: 0.0,
            });

        metrics.total_opportunities += 1;
        metrics.total_profit += &profit;
        
        if metrics.total_opportunities > 0 {
            metrics.average_profit = &metrics.total_profit / BigDecimal::from(metrics.total_opportunities);
        }
        
        if profit > metrics.best_profit {
            metrics.best_profit = profit;
        }

        // Update average price spread
        let total_spread = metrics.average_price_spread * (metrics.total_opportunities - 1) as f64;
        metrics.average_price_spread = (total_spread + price_spread) / metrics.total_opportunities as f64;

        // Calculate market efficiency (inverse of average spread)
        metrics.market_efficiency_score = if metrics.average_price_spread > 0.0 {
            1.0 / (1.0 + metrics.average_price_spread)
        } else {
            1.0
        };
    }

    pub fn generate_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("=== Arbitrage Bot Metrics Report ===\n");
        report.push_str(&format!("Uptime: {} seconds\n", self.uptime_seconds));
        report.push_str(&format!("Total Cycles: {}\n", self.total_cycles_completed));
        report.push_str(&format!("Opportunities Found: {}\n", self.total_opportunities_found));
        report.push_str(&format!("Total Simulated Profit: {} USDC\n", self.total_profit_simulated));
        report.push_str(&format!("Average Profit per Opportunity: {} USDC\n", self.average_profit_per_opportunity));
        report.push_str(&format!("Success Rate: {:.2}%\n", self.success_rate * 100.0));
        report.push_str(&format!("Error Count: {}\n", self.error_count));
        
        if let Some(ref error) = self.last_error {
            report.push_str(&format!("Last Error: {}\n", error));
        }
        
        report.push_str("\n=== DEX Performance ===\n");
        for (dex_name, metrics) in &self.dex_performance {
            report.push_str(&format!(
                "{}: {}/{} successful quotes ({:.1}% success rate), avg response: {:.1}ms\n",
                dex_name,
                metrics.successful_quotes,
                metrics.total_quotes_fetched,
                if metrics.total_quotes_fetched > 0 {
                    metrics.successful_quotes as f64 / metrics.total_quotes_fetched as f64 * 100.0
                } else { 0.0 },
                metrics.average_response_time_ms
            ));
        }
        
        report.push_str("\n=== Token Pair Performance ===\n");
        for (pair, metrics) in &self.token_pair_performance {
            report.push_str(&format!(
                "{}: {} opportunities, {} USDC total profit, {:.2}% avg spread\n",
                pair,
                metrics.total_opportunities,
                metrics.total_profit,
                metrics.average_price_spread * 100.0
            ));
        }
        
        report.push_str(&format!("\nLast Updated: {}\n", self.last_updated));
        
        report
    }

    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize metrics: {}", e))
    }

    pub fn reset(&mut self) {
        *self = Self::new();
        info!("Bot metrics reset");
    }
}

impl Default for BotMetrics {
    fn default() -> Self {
        Self::new()
    }
}
