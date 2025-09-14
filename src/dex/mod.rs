pub mod uniswap;
pub mod quickswap;
pub mod traits;

pub use traits::*;
pub use uniswap::UniswapV3Client;
pub use quickswap::QuickSwapClient;

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{blockchain::BlockchainClient, config::DexConfig, types::*};

pub struct DexManager {
    clients: Vec<Box<dyn DexClient>>,
}

impl DexManager {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    pub fn add_client(&mut self, client: Box<dyn DexClient>) {
        self.clients.push(client);
    }

    pub async fn get_all_prices(&self, token_pair: &TokenPair) -> Result<Vec<PriceQuote>> {
        let mut all_quotes = Vec::new();
        
        for client in &self.clients {
            match client.get_price(token_pair).await {
                Ok(quote) => all_quotes.push(quote),
                Err(e) => {
                    tracing::warn!(
                        "Failed to get price from {}: {}",
                        client.name(),
                        e
                    );
                }
            }
        }
        
        Ok(all_quotes)
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

pub fn create_dex_clients(
    blockchain_client: Arc<BlockchainClient>,
    dex_configs: &std::collections::HashMap<String, DexConfig>,
) -> Result<DexManager> {
    let mut manager = DexManager::new();
    
    for (key, config) in dex_configs {
        match key.as_str() {
            "uniswap" => {
                let client = UniswapV3Client::new(blockchain_client.clone(), config.clone())?;
                manager.add_client(Box::new(client));
            }
            "quickswap" => {
                let client = QuickSwapClient::new(blockchain_client.clone(), config.clone())?;
                manager.add_client(Box::new(client));
            }
            _ => {
                tracing::warn!("Unknown DEX configuration: {}", key);
            }
        }
    }
    
    Ok(manager)
}
