use anyhow::{anyhow, Result};
use std::{sync::Arc, time::Duration};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use crate::{
    arbitrage::{ArbitrageDetector, OpportunityAnalyzer, ProfitCalculator},
    blockchain::BlockchainClient,
    config::Config,
    database::{ArbitrageRepository, DatabaseConnection},
    dex::{create_dex_clients, DexManager},
    types::{ArbitrageOpportunity, TokenPair},
};

pub struct ArbitrageBot {
    config: Config,
    blockchain_client: Arc<BlockchainClient>,
    dex_manager: DexManager,
    arbitrage_detector: ArbitrageDetector,
    profit_calculator: ProfitCalculator,
    opportunity_analyzer: OpportunityAnalyzer,
    database: Arc<DatabaseConnection>,
    repository: ArbitrageRepository,
    is_running: bool,
}

impl ArbitrageBot {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing Arbitrage Bot");

        // Initialize blockchain client
        let blockchain_client = Arc::new(BlockchainClient::new(&config).await?);
        info!("Blockchain client initialized");

        // Initialize DEX clients
        let dex_manager = create_dex_clients(blockchain_client.clone(), &config.dexes)?;
        info!("DEX clients initialized: {} clients", dex_manager.client_count());

        // Initialize arbitrage components
        let arbitrage_detector = ArbitrageDetector::new(config.arbitrage.clone())?;
        let profit_calculator = ProfitCalculator::default();
        let opportunity_analyzer = OpportunityAnalyzer::new();

        // Initialize database
        let database = Arc::new(DatabaseConnection::new(&config.database).await?);
        database.run_migrations().await?;
        let repository = ArbitrageRepository::new(database.pool().clone());

        info!("Arbitrage Bot initialized successfully");

        Ok(Self {
            config,
            blockchain_client,
            dex_manager,
            arbitrage_detector,
            profit_calculator,
            opportunity_analyzer,
            database,
            repository,
            is_running: false,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Err(anyhow!("Bot is already running"));
        }

        info!("Starting Arbitrage Bot");
        self.is_running = true;

        // Perform initial health checks
        self.perform_health_checks().await?;

        // Start the main monitoring loop
        self.run_monitoring_loop().await?;

        Ok(())
    }

    pub async fn stop(&mut self) {
        info!("Stopping Arbitrage Bot");
        self.is_running = false;
    }

    async fn run_monitoring_loop(&mut self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(self.config.arbitrage.check_interval_seconds));
        let mut cycle_count = 0u64;

        info!(
            "Starting monitoring loop with {} second intervals",
            self.config.arbitrage.check_interval_seconds
        );

        while self.is_running {
            interval.tick().await;
            cycle_count += 1;

            debug!("Starting monitoring cycle #{}", cycle_count);

            match self.run_single_cycle().await {
                Ok(opportunities_found) => {
                    debug!(
                        "Monitoring cycle #{} completed successfully, found {} opportunities",
                        cycle_count, opportunities_found
                    );
                }
                Err(e) => {
                    error!("Error in monitoring cycle #{}: {}", cycle_count, e);
                    
                    // Add exponential backoff on errors
                    let backoff_duration = Duration::from_secs(30);
                    warn!("Backing off for {:?} due to error", backoff_duration);
                    sleep(backoff_duration).await;
                }
            }

            // Perform periodic maintenance
            if cycle_count % 100 == 0 {
                self.perform_maintenance().await?;
            }
        }

        info!("Monitoring loop stopped");
        Ok(())
    }

    async fn run_single_cycle(&mut self) -> Result<usize> {
        // Define token pairs to monitor
        let token_pairs = self.get_monitored_token_pairs();
        let mut total_opportunities = 0;

        for token_pair in token_pairs {
            match self.process_token_pair(&token_pair).await {
                Ok(opportunities) => {
                    total_opportunities += opportunities.len();
                    
                    // Save opportunities to database and analyzer
                    for opportunity in opportunities {
                        self.repository.save_opportunity(&opportunity).await?;
                        self.opportunity_analyzer.add_opportunity(opportunity);
                    }
                }
                Err(e) => {
                    warn!("Failed to process token pair {:?}: {}", token_pair, e);
                }
            }
        }

        Ok(total_opportunities)
    }

    async fn process_token_pair(&self, token_pair: &TokenPair) -> Result<Vec<ArbitrageOpportunity>> {
        debug!("Processing token pair: {}/{}", token_pair.token0_symbol, token_pair.token1_symbol);

        // Fetch prices from all DEXes
        let quotes = self.dex_manager.get_all_prices(token_pair).await?;
        
        if quotes.is_empty() {
            warn!("No price quotes available for token pair");
            return Ok(Vec::new());
        }

        debug!("Fetched {} price quotes", quotes.len());

        // Save price quotes to database
        for quote in &quotes {
            if let Err(e) = self.repository.save_price_quote(quote).await {
                warn!("Failed to save price quote: {}", e);
            }
        }

        // Detect arbitrage opportunities
        let opportunities = self.arbitrage_detector.detect_opportunities(&quotes)?;
        
        if !opportunities.is_empty() {
            info!(
                "Found {} arbitrage opportunities for {}/{}",
                opportunities.len(),
                token_pair.token0_symbol,
                token_pair.token1_symbol
            );

            // Log each opportunity
            for opportunity in &opportunities {
                info!(
                    "Arbitrage Opportunity: Buy {} at {} for {}, sell at {} for {}, net profit: {} USDC",
                    opportunity.token_pair.token0_symbol,
                    opportunity.buy_dex,
                    opportunity.buy_price,
                    opportunity.sell_dex,
                    opportunity.sell_price,
                    opportunity.net_profit
                );
            }
        }

        Ok(opportunities)
    }

    fn get_monitored_token_pairs(&self) -> Vec<TokenPair> {
        vec![
            TokenPair {
                token0: self.config.tokens.weth.clone(),
                token1: self.config.tokens.usdc.clone(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "USDC".to_string(),
            },
            TokenPair {
                token0: self.config.tokens.wbtc.clone(),
                token1: self.config.tokens.usdc.clone(),
                token0_symbol: "WBTC".to_string(),
                token1_symbol: "USDC".to_string(),
            },
            TokenPair {
                token0: self.config.tokens.weth.clone(),
                token1: self.config.tokens.wbtc.clone(),
                token0_symbol: "WETH".to_string(),
                token1_symbol: "WBTC".to_string(),
            },
        ]
    }

    async fn perform_health_checks(&self) -> Result<()> {
        info!("Performing health checks");

        // Check blockchain connection
        self.blockchain_client.health_check().await
            .map_err(|e| anyhow!("Blockchain health check failed: {}", e))?;

        // Check database connection
        self.database.health_check().await
            .map_err(|e| anyhow!("Database health check failed: {}", e))?;

        // Check DEX clients (simplified - would need to implement health check for each)
        if self.dex_manager.client_count() == 0 {
            return Err(anyhow!("No DEX clients available"));
        }

        info!("All health checks passed");
        Ok(())
    }

    async fn perform_maintenance(&mut self) -> Result<()> {
        info!("Performing periodic maintenance");

        // Clean up old data (keep last 30 days)
        match self.repository.cleanup_old_data(30).await {
            Ok((opportunities_deleted, quotes_deleted)) => {
                info!(
                    "Maintenance: Cleaned up {} old opportunities and {} old quotes",
                    opportunities_deleted, quotes_deleted
                );
            }
            Err(e) => {
                warn!("Failed to cleanup old data: {}", e);
            }
        }

        // Generate and log market analysis
        let analysis = self.opportunity_analyzer.generate_market_analysis();
        info!(
            "Market Analysis: {} total opportunities, avg profit: {}, efficiency: {:.2}%",
            analysis.total_opportunities_found,
            analysis.average_profit_per_opportunity,
            analysis.market_efficiency_score * 100.0
        );

        // Update gas cost estimates based on current network conditions
        match self.blockchain_client.get_gas_price().await {
            Ok(gas_price) => {
                let gas_cost_usd = self.estimate_gas_cost_usd(gas_price).await;
                // Update the detector's gas cost estimate if significantly different
                debug!("Current estimated gas cost: {} USD", gas_cost_usd);
            }
            Err(e) => {
                warn!("Failed to update gas cost estimate: {}", e);
            }
        }

        Ok(())
    }

    async fn estimate_gas_cost_usd(&self, gas_price_wei: ethers::types::U256) -> f64 {
        // Simplified gas cost estimation
        // In reality, this would need to fetch ETH/USD price and calculate more accurately
        let gas_limit = 200_000u64; // Estimated gas limit for arbitrage transaction
        let gas_cost_wei = gas_price_wei * ethers::types::U256::from(gas_limit);
        
        // Convert to ETH (simplified)
        let gas_cost_eth = gas_cost_wei.as_u64() as f64 / 1e18;
        
        // Assume ETH price of $2000 for simplification
        gas_cost_eth * 2000.0
    }

    pub fn get_stats(&self) -> BotStats {
        let analysis = self.opportunity_analyzer.generate_market_analysis();
        
        BotStats {
            is_running: self.is_running,
            total_opportunities_found: analysis.total_opportunities_found,
            average_profit: analysis.average_profit_per_opportunity,
            market_efficiency_score: analysis.market_efficiency_score,
            dex_client_count: self.dex_manager.client_count(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BotStats {
    pub is_running: bool,
    pub total_opportunities_found: u64,
    pub average_profit: bigdecimal::BigDecimal,
    pub market_efficiency_score: f64,
    pub dex_client_count: usize,
}
