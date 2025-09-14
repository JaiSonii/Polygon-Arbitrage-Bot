use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{
    database::models::*,
    types::{ArbitrageOpportunity, PriceQuote, TokenPair},
};

pub struct ArbitrageRepository {
    pool: PgPool,
}

impl ArbitrageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        let row = ArbitrageOpportunityRow::from(opportunity.clone());

        sqlx::query(
            r#"
            INSERT INTO arbitrage_opportunities (
                id, token0_address, token1_address, token0_symbol, token1_symbol,
                buy_dex, sell_dex, buy_price, sell_price, price_difference,
                price_difference_percentage, estimated_profit, trade_amount,
                gas_cost, net_profit, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
        )
        .bind(&row.id)
        .bind(&row.token0_address)
        .bind(&row.token1_address)
        .bind(&row.token0_symbol)
        .bind(&row.token1_symbol)
        .bind(&row.buy_dex)
        .bind(&row.sell_dex)
        .bind(&row.buy_price)
        .bind(&row.sell_price)
        .bind(&row.price_difference)
        .bind(&row.price_difference_percentage)
        .bind(&row.estimated_profit)
        .bind(&row.trade_amount)
        .bind(&row.gas_cost)
        .bind(&row.net_profit)
        .bind(&row.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to save arbitrage opportunity: {}", e))?;

        debug!("Saved arbitrage opportunity: {}", opportunity.id);
        Ok(())
    }

    pub async fn save_price_quote(&self, quote: &PriceQuote) -> Result<()> {
        let row = PriceQuoteRow::from(quote.clone());

        sqlx::query(
            r#"
            INSERT INTO price_quotes (
                dex_name, token0_address, token1_address, token0_symbol, token1_symbol,
                price, liquidity, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&row.dex_name)
        .bind(&row.token0_address)
        .bind(&row.token1_address)
        .bind(&row.token0_symbol)
        .bind(&row.token1_symbol)
        .bind(&row.price)
        .bind(&row.liquidity)
        .bind(&row.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to save price quote: {}", e))?;

        debug!("Saved price quote from {}", quote.dex_name);
        Ok(())
    }

    pub async fn get_opportunities_by_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let rows = sqlx::query_as::<_, ArbitrageOpportunityRow>(
            r#"
            SELECT * FROM arbitrage_opportunities
            WHERE timestamp BETWEEN $1 AND $2
            ORDER BY timestamp DESC
            "#,
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch opportunities by time range: {}", e))?;

        Ok(rows.into_iter().map(ArbitrageOpportunity::from).collect())
    }

    pub async fn get_recent_opportunities(&self, limit: i64) -> Result<Vec<ArbitrageOpportunity>> {
        let rows = sqlx::query_as::<_, ArbitrageOpportunityRow>(
            r#"
            SELECT * FROM arbitrage_opportunities
            ORDER BY timestamp DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch recent opportunities: {}", e))?;

        Ok(rows.into_iter().map(ArbitrageOpportunity::from).collect())
    }

    pub async fn get_opportunities_by_token_pair(
        &self,
        token_pair: &TokenPair,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let rows = sqlx::query_as::<_, ArbitrageOpportunityRow>(
            r#"
            SELECT * FROM arbitrage_opportunities
            WHERE (token0_address = $1 AND token1_address = $2)
               OR (token0_address = $2 AND token1_address = $1)
            ORDER BY timestamp DESC
            LIMIT 100
            "#,
        )
        .bind(&token_pair.token0)
        .bind(&token_pair.token1)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch opportunities by token pair: {}", e))?;

        Ok(rows.into_iter().map(ArbitrageOpportunity::from).collect())
    }

    pub async fn get_opportunity_stats(&self, days: i32) -> Result<OpportunityStats> {
        let start_time = Utc::now() - Duration::days(days as i64);

        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_opportunities,
                COALESCE(SUM(net_profit), 0) as total_profit,
                COALESCE(AVG(net_profit), 0) as average_profit,
                COALESCE(MAX(net_profit), 0) as best_opportunity_profit
            FROM arbitrage_opportunities
            WHERE timestamp >= $1
            "#,
        )
        .bind(start_time)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch opportunity stats: {}", e))?;

        let total_opportunities: i64 = row.try_get("total_opportunities")?;
        let total_profit: BigDecimal = row.try_get("total_profit")?;
        let average_profit: BigDecimal = row.try_get("average_profit")?;
        let best_opportunity_profit: BigDecimal = row.try_get("best_opportunity_profit")?;

        // Get most active DEX pair
        let most_active_dex_pair = self.get_most_active_dex_pair(start_time).await?;

        Ok(OpportunityStats {
            total_opportunities,
            total_profit,
            average_profit,
            best_opportunity_profit,
            most_active_dex_pair,
        })
    }

    pub async fn get_price_quotes_by_time_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        dex_name: Option<&str>,
    ) -> Result<Vec<PriceQuote>> {
        let query = if let Some(dex) = dex_name {
            sqlx::query_as::<_, PriceQuoteRow>(
                r#"
                SELECT * FROM price_quotes
                WHERE timestamp BETWEEN $1 AND $2 AND dex_name = $3
                ORDER BY timestamp DESC
                "#,
            )
            .bind(start_time)
            .bind(end_time)
            .bind(dex)
        } else {
            sqlx::query_as::<_, PriceQuoteRow>(
                r#"
                SELECT * FROM price_quotes
                WHERE timestamp BETWEEN $1 AND $2
                ORDER BY timestamp DESC
                "#,
            )
            .bind(start_time)
            .bind(end_time)
        };

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to fetch price quotes by time range: {}", e))?;

        Ok(rows.into_iter().map(PriceQuote::from).collect())
    }

    pub async fn cleanup_old_data(&self, days_to_keep: i32) -> Result<(u64, u64)> {
        let cutoff_time = Utc::now() - Duration::days(days_to_keep as i64);

        let opportunities_deleted = sqlx::query(
            "DELETE FROM arbitrage_opportunities WHERE timestamp < $1"
        )
        .bind(cutoff_time)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to cleanup old opportunities: {}", e))?
        .rows_affected();

        let quotes_deleted = sqlx::query(
            "DELETE FROM price_quotes WHERE timestamp < $1"
        )
        .bind(cutoff_time)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to cleanup old quotes: {}", e))?
        .rows_affected();

        info!(
            "Cleaned up {} old opportunities and {} old quotes",
            opportunities_deleted, quotes_deleted
        );

        Ok((opportunities_deleted, quotes_deleted))
    }

    async fn get_most_active_dex_pair(&self, since: DateTime<Utc>) -> Result<Option<(String, String)>> {
        let row = sqlx::query(
            r#"
            SELECT buy_dex, sell_dex, COUNT(*) as count
            FROM arbitrage_opportunities
            WHERE timestamp >= $1
            GROUP BY buy_dex, sell_dex
            ORDER BY count DESC
            LIMIT 1
            "#,
        )
        .bind(since)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch most active DEX pair: {}", e))?;

        if let Some(row) = row {
            let buy_dex: String = row.try_get("buy_dex")?;
            let sell_dex: String = row.try_get("sell_dex")?;
            Ok(Some((buy_dex, sell_dex)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_dex_performance_stats(&self, days: i32) -> Result<Vec<DexStats>> {
        let start_time = Utc::now() - Duration::days(days as i64);

        let rows = sqlx::query(
            r#"
            SELECT 
                dex_name,
                COUNT(*) as total_quotes,
                AVG(price) as average_price,
                STDDEV(price) as price_volatility,
                MAX(timestamp) as last_update
            FROM price_quotes
            WHERE timestamp >= $1
            GROUP BY dex_name
            ORDER BY total_quotes DESC
            "#,
        )
        .bind(start_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to fetch DEX performance stats: {}", e))?;

        let mut stats = Vec::new();
        for row in rows {
            let dex_name: String = row.try_get("dex_name")?;
            let total_quotes: i64 = row.try_get("total_quotes")?;
            let average_price: Option<BigDecimal> = row.try_get("average_price")?;
            let price_volatility: Option<BigDecimal> = row.try_get("price_volatility")?;
            let last_update: Option<DateTime<Utc>> = row.try_get("last_update")?;

            stats.push(DexStats {
                dex_name,
                total_quotes,
                average_price: average_price.unwrap_or_else(|| BigDecimal::from(0)),
                price_volatility: price_volatility.unwrap_or_else(|| BigDecimal::from(0)),
                last_update: last_update.unwrap_or_else(|| Utc::now()),
            });
        }

        Ok(stats)
    }
}
