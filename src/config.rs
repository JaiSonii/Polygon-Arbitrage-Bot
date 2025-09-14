use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub blockchain: BlockchainConfig,
    pub tokens: TokenConfig,
    pub dexes: HashMap<String, DexConfig>,
    pub arbitrage: ArbitrageConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    pub chain_id: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenConfig {
    pub weth: String,
    pub usdc: String,
    pub wbtc: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DexConfig {
    pub name: String,
    pub router_address: String,
    pub factory_address: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArbitrageConfig {
    pub min_profit_threshold: String,
    pub trade_amount: String,
    pub gas_cost_estimate: String,
    pub check_interval_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        dotenv::dotenv().ok();
        
        let mut settings = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::Environment::with_prefix("ARBITRAGE"));

        // Override database URL from environment if present
        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            settings = settings.set_override("database.url", db_url)?;
        }

        // Override RPC URL from environment if present
        if let Ok(rpc_url) = std::env::var("POLYGON_RPC_URL") {
            settings = settings.set_override("blockchain.rpc_url", rpc_url)?;
        }

        let config = settings.build()?.try_deserialize()?;
        Ok(config)
    }
}
