use anyhow::Result;
use async_trait::async_trait;

use crate::types::{PriceQuote, TokenPair};

#[async_trait]
pub trait DexClient: Send + Sync {
    fn name(&self) -> &str;
    
    async fn get_price(&self, token_pair: &TokenPair) -> Result<PriceQuote>;
    
    async fn get_liquidity(&self, token_pair: &TokenPair) -> Result<Option<bigdecimal::BigDecimal>>;
    
    async fn health_check(&self) -> Result<()>;
}
