use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::Utc;
use ethers::{
    abi::{Abi, Token},
    contract::Contract,
    prelude::*,
    types::{Address, U256},
};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::{
    blockchain::{parse_address, BlockchainClient},
    config::DexConfig,
    dex::traits::DexClient,
    types::{PriceQuote, TokenPair},
};

pub struct UniswapV3Client {
    blockchain_client: Arc<BlockchainClient>,
    config: DexConfig,
    quoter_contract: Contract<Arc<Provider<Http>>>,
}

impl UniswapV3Client {
    pub fn new(blockchain_client: Arc<BlockchainClient>, config: DexConfig) -> Result<Self> {
        let quoter_address = parse_address("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6")?; // Uniswap V3 Quoter
        
        // Simplified ABI for the quoter contract
        let quoter_abi: Abi = serde_json::from_str(r#"
        [
            {
                "inputs": [
                    {"internalType": "address", "name": "tokenIn", "type": "address"},
                    {"internalType": "address", "name": "tokenOut", "type": "address"},
                    {"internalType": "uint24", "name": "fee", "type": "uint24"},
                    {"internalType": "uint256", "name": "amountIn", "type": "uint256"},
                    {"internalType": "uint160", "name": "sqrtPriceLimitX96", "type": "uint160"}
                ],
                "name": "quoteExactInputSingle",
                "outputs": [
                    {"internalType": "uint256", "name": "amountOut", "type": "uint256"}
                ],
                "stateMutability": "nonpayable",
                "type": "function"
            }
        ]
        "#)?;

        let quoter_contract = Contract::new(
            quoter_address,
            quoter_abi,
            blockchain_client.provider(),
        );

        Ok(Self {
            blockchain_client,
            config,
            quoter_contract,
        })
    }

    async fn get_quote_for_amount(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        fee_tier: u32,
    ) -> Result<U256> {
        let call = self.quoter_contract.method::<_, U256>(
            "quoteExactInputSingle",
            (token_in, token_out, fee_tier, amount_in, U256::zero()),
        )?;

        let amount_out = call.call().await.map_err(|e| {
            anyhow!("Failed to get quote from Uniswap V3: {}", e)
        })?;

        Ok(amount_out)
    }

    fn calculate_price_from_quote(&self, amount_in: U256, amount_out: U256) -> Result<BigDecimal> {
        if amount_in.is_zero() {
            return Err(anyhow!("Amount in cannot be zero"));
        }

        // Convert U256 to BigDecimal for precise calculations
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
impl DexClient for UniswapV3Client {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn get_price(&self, token_pair: &TokenPair) -> Result<PriceQuote> {
        debug!("Getting price from Uniswap V3 for {}/{}", 
               token_pair.token0_symbol, token_pair.token1_symbol);

        let token0_address = parse_address(&token_pair.token0)?;
        let token1_address = parse_address(&token_pair.token1)?;

        // Use 1 token (with 18 decimals) as the base amount for price calculation
        let base_amount = U256::from(10).pow(U256::from(18));
        
        // Try different fee tiers (0.05%, 0.3%, 1%)
        let fee_tiers = [500u32, 3000u32, 10000u32];
        let mut best_quote = None;
        let mut best_price = BigDecimal::from(0);

        for &fee_tier in &fee_tiers {
            match self.get_quote_for_amount(
                token0_address,
                token1_address,
                base_amount,
                fee_tier,
            ).await {
                Ok(amount_out) => {
                    if let Ok(price) = self.calculate_price_from_quote(base_amount, amount_out) {
                        if price > best_price {
                            best_price = price.clone();
                            best_quote = Some((amount_out, fee_tier));
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to get quote for fee tier {}: {}", fee_tier, e);
                }
            }
        }

        if best_quote.is_none() {
            return Err(anyhow!("No valid quotes found for token pair"));
        }

        Ok(PriceQuote {
            dex_name: self.config.name.clone(),
            token_pair: token_pair.clone(),
            price: best_price,
            timestamp: Utc::now(),
            liquidity: None, // We'll implement liquidity fetching separately if needed
        })
    }

    async fn get_liquidity(&self, _token_pair: &TokenPair) -> Result<Option<BigDecimal>> {
        // Placeholder for liquidity calculation
        // This would require additional contract calls to get pool reserves
        Ok(None)
    }

    async fn health_check(&self) -> Result<()> {
        debug!("Performing Uniswap V3 health check");
        
        // Try to call a simple view function to verify the contract is accessible
        let weth_address = parse_address("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619")?;
        let usdc_address = parse_address("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174")?;
        let test_amount = U256::from(10).pow(U256::from(18));

        self.get_quote_for_amount(weth_address, usdc_address, test_amount, 3000)
            .await
            .map_err(|e| anyhow!("Uniswap V3 health check failed: {}", e))?;

        debug!("Uniswap V3 health check passed");
        Ok(())
    }
}
