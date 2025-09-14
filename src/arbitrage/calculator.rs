use anyhow::Result;
use bigdecimal::BigDecimal;
use tracing::debug;

use crate::types::ArbitrageOpportunity;

pub struct ProfitCalculator {
    slippage_tolerance: BigDecimal,
    additional_fees: BigDecimal,
}

impl ProfitCalculator {
    pub fn new(slippage_tolerance_percent: f64, additional_fees: BigDecimal) -> Self {
        Self {
            slippage_tolerance: BigDecimal::from(slippage_tolerance_percent) / BigDecimal::from(100),
            additional_fees,
        }
    }

    pub fn calculate_realistic_profit(
        &self,
        opportunity: &ArbitrageOpportunity,
    ) -> Result<BigDecimal> {
        // Account for slippage on both buy and sell sides
        let buy_price_with_slippage = &opportunity.buy_price * (BigDecimal::from(1) + &self.slippage_tolerance);
        let sell_price_with_slippage = &opportunity.sell_price * (BigDecimal::from(1) - &self.slippage_tolerance);

        // Calculate profit with slippage
        let price_difference_with_slippage = sell_price_with_slippage - buy_price_with_slippage;
        let gross_profit = price_difference_with_slippage * &opportunity.trade_amount;

        // Subtract gas costs and additional fees
        let net_profit = gross_profit - &opportunity.gas_cost - &self.additional_fees;

        debug!(
            "Realistic profit calculation: gross={}, gas={}, fees={}, net={}",
            gross_profit, opportunity.gas_cost, self.additional_fees, net_profit
        );

        Ok(net_profit)
    }

    pub fn calculate_roi(&self, opportunity: &ArbitrageOpportunity) -> Result<BigDecimal> {
        let investment = &opportunity.trade_amount * &opportunity.buy_price;
        
        if investment <= BigDecimal::from(0) {
            return Ok(BigDecimal::from(0));
        }

        let roi = (&opportunity.net_profit / investment) * BigDecimal::from(100);
        Ok(roi)
    }

    pub fn calculate_break_even_price(&self, opportunity: &ArbitrageOpportunity) -> Result<BigDecimal> {
        // Calculate the minimum sell price needed to break even
        let total_costs = &opportunity.gas_cost + &self.additional_fees;
        let break_even_price = &opportunity.buy_price + (total_costs / &opportunity.trade_amount);
        
        Ok(break_even_price)
    }

    pub fn estimate_execution_time(&self, opportunity: &ArbitrageOpportunity) -> u64 {
        // Simple estimation based on trade amount and typical block times
        // This is a placeholder - in reality, this would depend on network congestion,
        // gas price, and DEX-specific factors
        
        let base_time_seconds = 30u64; // Base execution time
        let amount_factor = if opportunity.trade_amount > BigDecimal::from(10000) {
            2 // Larger trades might take longer
        } else {
            1
        };
        
        base_time_seconds * amount_factor
    }

    pub fn calculate_price_impact(&self, trade_amount: &BigDecimal, liquidity: Option<&BigDecimal>) -> BigDecimal {
        match liquidity {
            Some(liq) if *liq > BigDecimal::from(0) => {
                // Simple price impact estimation: impact = trade_amount / liquidity
                // This is a simplified model - real price impact is more complex
                let impact = trade_amount / liq;
                // Cap the impact at 10% for safety
                if impact > BigDecimal::from(0.1) {
                    BigDecimal::from(0.1)
                } else {
                    impact
                }
            }
            _ => BigDecimal::from(0.01), // Default 1% impact if liquidity is unknown
        }
    }

    pub fn adjust_for_market_conditions(
        &self,
        opportunity: &mut ArbitrageOpportunity,
        market_volatility: f64,
    ) -> Result<()> {
        // Adjust gas cost based on network congestion (simplified)
        let volatility_multiplier = BigDecimal::from(1.0 + market_volatility);
        opportunity.gas_cost = &opportunity.gas_cost * volatility_multiplier;

        // Recalculate net profit
        opportunity.net_profit = &opportunity.estimated_profit - &opportunity.gas_cost;

        debug!(
            "Adjusted opportunity for market conditions: volatility={}, new_gas_cost={}, new_net_profit={}",
            market_volatility, opportunity.gas_cost, opportunity.net_profit
        );

        Ok(())
    }
}

impl Default for ProfitCalculator {
    fn default() -> Self {
        Self::new(0.5, BigDecimal::from(1.0)) // 0.5% slippage, $1 additional fees
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TokenPair;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_opportunity() -> ArbitrageOpportunity {
        ArbitrageOpportunity {
            id: Uuid::new_v4(),
            token_pair: TokenPair {
                token0: "0x123".to_string(),
                token1: "0x456".to_string(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "USDC".to_string(),
            },
            buy_dex: "Uniswap".to_string(),
            sell_dex: "QuickSwap".to_string(),
            buy_price: BigDecimal::from(2000),
            sell_price: BigDecimal::from(2010),
            price_difference: BigDecimal::from(10),
            price_difference_percentage: BigDecimal::from(0.5),
            estimated_profit: BigDecimal::from(10000), // 1000 * 10
            trade_amount: BigDecimal::from(1000),
            gas_cost: BigDecimal::from(5),
            net_profit: BigDecimal::from(9995),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_calculate_realistic_profit() {
        let calculator = ProfitCalculator::new(0.5, BigDecimal::from(2.0));
        let opportunity = create_test_opportunity();

        let realistic_profit = calculator.calculate_realistic_profit(&opportunity).unwrap();
        
        // With 0.5% slippage on both sides, the profit should be less than the original
        assert!(realistic_profit < opportunity.net_profit);
    }

    #[test]
    fn test_calculate_roi() {
        let calculator = ProfitCalculator::default();
        let opportunity = create_test_opportunity();

        let roi = calculator.calculate_roi(&opportunity).unwrap();
        
        // ROI should be positive for a profitable opportunity
        assert!(roi > BigDecimal::from(0));
    }

    #[test]
    fn test_calculate_break_even_price() {
        let calculator = ProfitCalculator::default();
        let opportunity = create_test_opportunity();

        let break_even = calculator.calculate_break_even_price(&opportunity).unwrap();
        
        // Break-even price should be higher than buy price
        assert!(break_even > opportunity.buy_price);
    }
}
