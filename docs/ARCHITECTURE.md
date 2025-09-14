# Architecture Documentation

## System Overview

The Polygon Arbitrage Opportunity Detector Bot is designed as a modular, event-driven system that continuously monitors multiple DEXes for arbitrage opportunities.

## Core Components

### 1. Blockchain Client (`src/blockchain.rs`)

**Purpose**: Manages connection to the Polygon network via RPC.

**Key Features**:
- Connection management with health checks
- Gas price estimation
- Contract interaction utilities
- Address parsing and validation

**Dependencies**: 
- `ethers` for Ethereum/Polygon interaction
- HTTP provider for RPC communication

### 2. DEX Clients (`src/dex/`)

**Purpose**: Abstracts price fetching from different DEXes.

**Architecture**:
```rust
trait DexClient {
    async fn get_price(&self, token_pair: &TokenPair) -> Result<PriceQuote>;
    async fn get_liquidity(&self, token_pair: &TokenPair) -> Result<Option<BigDecimal>>;
    async fn health_check(&self) -> Result<()>;
}
