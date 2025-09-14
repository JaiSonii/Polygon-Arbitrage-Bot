use anyhow::{anyhow, Result};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use tracing::{info, warn};

use crate::config::DatabaseConfig;

pub struct DatabaseConnection {
    pool: PgPool,
}

impl DatabaseConnection {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Connecting to database: {}", mask_database_url(&config.url));

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&config.url)
            .await
            .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

        // Test the connection
        let connection = Self { pool };
        connection.health_check().await?;

        info!("Successfully connected to database");
        Ok(connection)
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn health_check(&self) -> Result<()> {
        let row = sqlx::query("SELECT 1 as test")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("Database health check failed: {}", e))?;

        let test_value: i32 = row.try_get("test")?;
        if test_value != 1 {
            return Err(anyhow!("Database health check returned unexpected value"));
        }

        Ok(())
    }

    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations");

        // Create arbitrage_opportunities table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS arbitrage_opportunities (
                id UUID PRIMARY KEY,
                token0_address VARCHAR(42) NOT NULL,
                token1_address VARCHAR(42) NOT NULL,
                token0_symbol VARCHAR(10) NOT NULL,
                token1_symbol VARCHAR(10) NOT NULL,
                buy_dex VARCHAR(50) NOT NULL,
                sell_dex VARCHAR(50) NOT NULL,
                buy_price DECIMAL(36, 18) NOT NULL,
                sell_price DECIMAL(36, 18) NOT NULL,
                price_difference DECIMAL(36, 18) NOT NULL,
                price_difference_percentage DECIMAL(10, 4) NOT NULL,
                estimated_profit DECIMAL(36, 18) NOT NULL,
                trade_amount DECIMAL(36, 18) NOT NULL,
                gas_cost DECIMAL(36, 18) NOT NULL,
                net_profit DECIMAL(36, 18) NOT NULL,
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create arbitrage_opportunities table: {}", e))?;

        // Create price_quotes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS price_quotes (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                dex_name VARCHAR(50) NOT NULL,
                token0_address VARCHAR(42) NOT NULL,
                token1_address VARCHAR(42) NOT NULL,
                token0_symbol VARCHAR(10) NOT NULL,
                token1_symbol VARCHAR(10) NOT NULL,
                price DECIMAL(36, 18) NOT NULL,
                liquidity DECIMAL(36, 18),
                timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Failed to create price_quotes table: {}", e))?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_arbitrage_opportunities_timestamp ON arbitrage_opportunities(timestamp)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create timestamp index: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_arbitrage_opportunities_tokens ON arbitrage_opportunities(token0_address, token1_address)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create tokens index: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_price_quotes_timestamp ON price_quotes(timestamp)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create price quotes timestamp index: {}", e))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_price_quotes_dex_tokens ON price_quotes(dex_name, token0_address, token1_address)")
            .execute(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to create price quotes dex tokens index: {}", e))?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection closed");
    }
}

fn mask_database_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let mut masked = url.to_string();
            masked.replace_range(colon_pos + 1..at_pos, "****");
            return masked;
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_database_url() {
        let url = "postgresql://user:password@localhost/db";
        let masked = mask_database_url(url);
        assert_eq!(masked, "postgresql://user:****@localhost/db");
    }

    #[test]
    fn test_mask_database_url_no_password() {
        let url = "postgresql://localhost/db";
        let masked = mask_database_url(url);
        assert_eq!(masked, "postgresql://localhost/db");
    }
}
