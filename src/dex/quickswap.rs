use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::Utc;
use ethers::{
    abi::Abi,
    contract::Contract,
    prelude::*,
    types::{Address, U256},
};
use std::sync::Arc;
use tracing::{debug, error};

use crate::{
    blockchain::{parse_address, BlockchainClient},
    config::DexConfig,
    dex::traits::DexClient,
    types::{PriceQuote, TokenPair},
};

pub struct QuickSwapClient {
    blockchain_client: Arc<BlockchainClient>,
    config: DexConfig,
    router_contract: Contract<Arc<Provider<Http>>>,
}

impl QuickSwapClient {
    pub fn new(blockchain_client: Arc<BlockchainClient>, config: DexConfig) -> Result<Self> {
        let router_address = parse_address(&config.router_address)?;
        
        // Simplified ABI for QuickSwap router (Uniswap V2 compatible)
        let router_abi: Abi = serde_json::from_str(r#"
        [
            {
                "inputs": [
                    {"internalType": "uint256", "name": "amountIn", "type": "uint256"},
                    {"internalType": "address[]", "name": "path", "type": "address[]"}
                ],
                "name": "getAmountsOut",
                "outputs": [
                    {"internalType": "uint256[]", "name": "amounts", "type": "uint256[]"}
                ],
                "stateMutability": "view",
                "type": "function"
            },
            {
                "inputs": [
                    {"internalType": "address", "name": "tokenA", "type": "address"},
                    {"internalType": "address", "name": "tokenB", "type": "address"}
                ],
                "name": "getReserves",
                "outputs": [
                    {"internalType": "uint256", "name": "reserveA", "type": "uint256"},
                    {"internalType": "uint256", "name": "reserveB", "type": "uint256"}
                ],
                "stateMutability": "view",
                "type": "function"
            }
        ]
        "#)?;

        let router_contract = Contract::new(
            router_address,
            router_abi,
            blockchain_client.provider(),
        );

        Ok(Self {
            blockchain_client,
            config,
            router_contract,
        })
    }

    async fn get_amounts_out(&self, amount_in: U256, path: Vec<Address>) -> Result<Vec<U256>> {
        let call = self.router_contract.method::<_, Vec<U256>>(
            "getAmountsOut",
            (amount_in, path),
        )?;

        let amounts = call.call().await.map_err(|e| {
            anyhow!("Failed to get amounts out from QuickSwap: {}", e)
        })?;

        Ok(amounts)
    }

    fn calculate_price_from_amounts(&self, amount_in: U256, amount_out: U256) -> Result<BigDecimal> {
        if amount_in.is_zero() {
            return Err(anyhow!("Amount in cannot be zero"));
        }

        let amount_in_str = amount_in.to_string();
        let amount_out_str = amount_out.to_string();
        
        let amount_in_bd = amount_in_str.parse::<BigDecimal>()?;
        let amount_out_bd = amount_out_str.parse::<BigDecimal>()?;

        if amount_in_bd.is_zero() {
            return Err(anyhow!("Amount in BigDecimal cannot be zero"));
        }

        let price = amount_out_bd / amount_in_bd;
        Ok(price)
    }
}

#[async_trait]
impl DexClient for QuickSwapClient {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn get_price(&self, token_pair: &TokenPair) -> Result<PriceQuote> {
        debug!("Getting price from QuickSwap for {}/{}", 
               token_pair.token0_symbol, token_pair.token1_symbol);

        let token0_address = parse_address(&token_pair.token0)?;
        let token1_address = parse_address(&token_pair.token1)?;

        // Use 1 token (with 18 decimals) as the base amount
        let base_amount = U256::from(10).pow(U256::from(18));
        let path = vec![token0_address, token1_address];

        let amounts = self.get_amounts_out(base_amount, path).await?;
        
        if amounts.len() < 2 {
            return Err(anyhow!("Invalid amounts returned from QuickSwap"));
        }

        let amount_out = amounts[1];
        let price = self.calculate_price_from_amounts(base_amount, amount_out)?;

        Ok(PriceQuote {
            dex_name: self.config.name.clone(),
            token_pair: token_pair.clone(),
            price,
            timestamp: Utc::now(),
            liquidity: None,
        })
    }

    async fn get_liquidity(&self, _token_pair: &TokenPair) -> Result<Option<BigDecimal>> {
        // Placeholder for liquidity calculation
        // This would require calls to the pair contract to get reserves
        Ok(None)
    }

    async fn health_check(&self) -> Result<()> {
        debug!("Performing QuickSwap health check");
        
        // Try to get amounts for a simple WETH -> USDC swap
        let weth_address = parse_address("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619")?;
        let usdc_address = parse_address("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174")?;
        let test_amount = U256::from(10).pow(U256::from(18));
        let path = vec![weth_address, usdc_address];

        self.get_amounts_out(test_amount, path)
            .await
            .map_err(|e| anyhow!("QuickSwap health check failed: {}", e))?;

        debug!("QuickSwap health check passed");
        Ok(())
    }
}
