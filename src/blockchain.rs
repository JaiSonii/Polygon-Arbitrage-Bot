use anyhow::{anyhow, Result};
use ethers::{
    prelude::*,
    providers::{Http, Provider},
    types::{Address, U256},
};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::config::Config;

pub struct BlockchainClient {
    provider: Arc<Provider<Http>>,
    chain_id: u64,
}

impl BlockchainClient {
    pub async fn new(config: &Config) -> Result<Self> {
        info!("Connecting to Polygon RPC: {}", config.blockchain.rpc_url);
        
        let provider = Provider::<Http>::try_from(&config.blockchain.rpc_url)
            .map_err(|e| anyhow!("Failed to create provider: {}", e))?;
        
        let provider = Arc::new(provider);
        
        // Verify connection by getting chain ID
        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| anyhow!("Failed to get chain ID: {}", e))?;
        
        if chain_id.as_u64() != config.blockchain.chain_id {
            return Err(anyhow!(
                "Chain ID mismatch: expected {}, got {}",
                config.blockchain.chain_id,
                chain_id.as_u64()
            ));
        }
        
        info!("Successfully connected to Polygon network (Chain ID: {})", chain_id);
        
        Ok(Self {
            provider,
            chain_id: chain_id.as_u64(),
        })
    }

    pub fn provider(&self) -> Arc<Provider<Http>> {
        self.provider.clone()
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub async fn get_block_number(&self) -> Result<U256> {
        self.provider
            .get_block_number()
            .await
            .map_err(|e| anyhow!("Failed to get block number: {}", e))
    }

    pub async fn get_gas_price(&self) -> Result<U256> {
        self.provider
            .get_gas_price()
            .await
            .map_err(|e| anyhow!("Failed to get gas price: {}", e))
    }

    pub async fn call_contract<T: Detokenize>(
        &self,
        contract_address: Address,
        function_call: FunctionCall<Arc<Provider<Http>>, Provider<Http>, T>,
    ) -> Result<T> {
        function_call
            .call()
            .await
            .map_err(|e| anyhow!("Contract call failed: {}", e))
    }

    pub async fn estimate_gas_cost(&self, gas_limit: U256) -> Result<U256> {
        let gas_price = self.get_gas_price().await?;
        Ok(gas_price * gas_limit)
    }

    pub async fn health_check(&self) -> Result<()> {
        debug!("Performing blockchain health check");
        
        let block_number = self.get_block_number().await?;
        let gas_price = self.get_gas_price().await?;
        
        debug!(
            "Health check passed - Block: {}, Gas Price: {} wei",
            block_number,
            gas_price
        );
        
        Ok(())
    }
}

// Utility functions for address parsing and validation
pub fn parse_address(address_str: &str) -> Result<Address> {
    address_str
        .parse::<Address>()
        .map_err(|e| anyhow!("Invalid address format '{}': {}", address_str, e))
}

pub fn format_address(address: &Address) -> String {
    format!("{:?}", address)
}

// Helper function to convert between different numeric types
pub fn u256_to_f64(value: U256) -> f64 {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    
    // Convert to f64 (this is a simplified conversion, may lose precision for very large numbers)
    let mut result = 0.0f64;
    for (i, &byte) in bytes.iter().enumerate() {
        result += (byte as f64) * 256.0f64.powi(31 - i as i32);
    }
    result
}

pub fn wei_to_ether(wei: U256) -> f64 {
    u256_to_f64(wei) / 1e18
}

pub fn wei_to_gwei(wei: U256) -> f64 {
    u256_to_f64(wei) / 1e9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address() {
        let valid_address = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619";
        assert!(parse_address(valid_address).is_ok());

        let invalid_address = "invalid_address";
        assert!(parse_address(invalid_address).is_err());
    }

    #[test]
    fn test_wei_conversions() {
        let one_ether_wei = U256::from(1_000_000_000_000_000_000u64);
        assert_eq!(wei_to_ether(one_ether_wei), 1.0);

        let one_gwei_wei = U256::from(1_000_000_000u64);
        assert_eq!(wei_to_gwei(one_gwei_wei), 1.0);
    }
}
