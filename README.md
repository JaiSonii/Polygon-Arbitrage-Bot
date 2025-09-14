# Polygon Arbitrage Opportunity Detector Bot

A sophisticated Rust-based bot that detects and analyzes arbitrage opportunities on the Polygon network by monitoring price differences across multiple Decentralized Exchanges (DEXes).

## 🎯 Overview

This bot continuously monitors token prices across different DEXes on Polygon (Uniswap V3, QuickSwap) and identifies profitable arbitrage opportunities. It simulates potential profits, accounts for gas costs, and stores historical data for analysis.

## ✨ Features

- **Multi-DEX Price Monitoring**: Fetches real-time prices from Uniswap V3 and QuickSwap
- **Arbitrage Detection**: Identifies profitable price differences across DEXes
- **Profit Simulation**: Calculates estimated profits accounting for gas costs and slippage
- **Database Storage**: Stores opportunities and price quotes for historical analysis
- **Market Analysis**: Provides insights into market efficiency and DEX performance
- **Configurable Parameters**: Customizable profit thresholds, trade amounts, and monitoring intervals
- **Comprehensive Logging**: Detailed logging with structured output
- **Health Monitoring**: Built-in health checks for all components

## 🏗️ Architecture

![Architecture](./architecture.png)


## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ 
- PostgreSQL 12+
- Access to Polygon RPC endpoint

### Installation

1. **Clone the repository**
   \`\`\`bash
   git clone <repository-url>
   cd polygon-arbitrage-bot
   \`\`\`

2. **Set up environment variables**
   \`\`\`bash
   cp .env.example .env
   # Edit .env with your configuration
   \`\`\`

3. **Configure the database**
   \`\`\`bash
   # Create PostgreSQL database
   createdb arbitrage_bot
   
   # Update DATABASE_URL in .env
   export DATABASE_URL="postgresql://username:password@localhost/arbitrage_bot"
   \`\`\`

4. **Build and run**
   \`\`\`bash
   cargo build --release
   cargo run
   \`\`\`

## ⚙️ Configuration

The bot uses a hierarchical configuration system with TOML files and environment variables.

### Configuration Files

- `config/default.toml` - Base configuration
- Environment variables override config file values

### Key Configuration Sections

#### Blockchain Settings
\`\`\`toml
[blockchain]
rpc_url = "https://polygon-rpc.com"
chain_id = 137
\`\`\`

#### Token Addresses
\`\`\`toml
[tokens]
weth = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"
usdc = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"
wbtc = "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6"
\`\`\`

#### DEX Configuration
\`\`\`toml
[dexes.uniswap]
name = "Uniswap V3"
router_address = "0xE592427A0AEce92De3Edee1F18E0157C05861564"
factory_address = "0x1F98431c8aD98523631AE4a59f267346ea31F984"

[dexes.quickswap]
name = "QuickSwap"
router_address = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"
factory_address = "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32"
\`\`\`

#### Arbitrage Parameters
\`\`\`toml
[arbitrage]
min_profit_threshold = "10.0"  # Minimum profit in USDC
trade_amount = "1000.0"        # Trade amount in USDC
gas_cost_estimate = "5.0"      # Estimated gas cost in USDC
check_interval_seconds = 30    # How often to check for opportunities
\`\`\`

### Environment Variables

- `DATABASE_URL` - PostgreSQL connection string
- `POLYGON_RPC_URL` - Polygon RPC endpoint
- `RUST_LOG` - Logging level (debug, info, warn, error)

## 📊 Database Schema

The bot creates and manages two main tables:

### arbitrage_opportunities
Stores detected arbitrage opportunities with profit calculations.

### price_quotes
Stores historical price data from all monitored DEXes.

## 🔍 Monitoring Token Pairs

The bot currently monitors these token pairs:
- WETH/USDC
- WBTC/USDC  
- WETH/WBTC

Additional pairs can be configured by modifying the `get_monitored_token_pairs()` function.

## 📈 Metrics and Analysis

The bot provides comprehensive metrics including:

- **Opportunity Metrics**: Total opportunities found, average profit, success rate
- **DEX Performance**: Response times, success rates, profit contributions
- **Token Pair Analysis**: Most profitable pairs, market efficiency scores
- **Market Analysis**: Overall market efficiency and trends

## 🛠️ Development

### Project Structure

\`\`\`
src/
├── arbitrage/          # Arbitrage detection logic
│   ├── detector.rs     # Main detection algorithm
│   ├── calculator.rs   # Profit calculations
│   └── analyzer.rs     # Market analysis
├── blockchain/         # Blockchain interaction
├── bot/               # Main bot orchestration
│   ├── orchestrator.rs # Main bot logic
│   ├── scheduler.rs    # Command/event handling
│   └── metrics.rs      # Performance metrics
├── config/            # Configuration management
├── database/          # Database models and operations
├── dex/              # DEX client implementations
│   ├── uniswap.rs    # Uniswap V3 client
│   ├── quickswap.rs  # QuickSwap client
│   └── traits.rs     # Common DEX interfaces
└── types/            # Common data structures
\`\`\`

### Running Tests

\`\`\`bash
cargo test
\`\`\`

### Adding New DEXes

1. Implement the `DexClient` trait in `src/dex/traits.rs`
2. Add your DEX client to `src/dex/mod.rs`
3. Update the `create_dex_clients` function
4. Add configuration in `config/default.toml`

### Database Migrations

The bot automatically runs migrations on startup. Manual migration files are in `migrations/`.

## 🔧 Troubleshooting

### Common Issues

1. **Database Connection Failed**
   - Verify PostgreSQL is running
   - Check DATABASE_URL format
   - Ensure database exists

2. **RPC Connection Issues**
   - Verify Polygon RPC endpoint is accessible
   - Check network connectivity
   - Try alternative RPC providers

3. **No Opportunities Found**
   - Markets may be efficient (normal)
   - Check if DEX clients are working
   - Verify token addresses are correct
   - Lower profit threshold for testing

4. **High Gas Cost Estimates**
   - Update gas cost estimates in configuration
   - Monitor network congestion
   - Adjust profit thresholds accordingly

### Logging

Enable debug logging for detailed information:
\`\`\`bash
RUST_LOG=debug cargo run
\`\`\`

## 📋 Performance Considerations

- **RPC Rate Limits**: Be mindful of RPC provider rate limits
- **Database Performance**: Regular cleanup of old data is performed automatically
- **Memory Usage**: The bot maintains limited in-memory caches
- **Network Latency**: Consider running close to Polygon network for best performance

## 🔒 Security Notes

- **Private Keys**: This bot does NOT execute trades, only simulates opportunities
- **RPC Security**: Use secure RPC endpoints
- **Database Security**: Secure your PostgreSQL instance
- **Network Security**: Consider running in a secure environment