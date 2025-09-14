use anyhow::Result;
use polygon_arbitrage_bot::{bot::ArbitrageBot, config::Config};
use tracing::{error, info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting Polygon Arbitrage Opportunity Detector Bot");

    // Load configuration
    let config = Config::load().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Configuration loaded successfully");

    // Initialize and start the bot
    let mut bot = ArbitrageBot::new(config).await.map_err(|e| {
        error!("Failed to initialize bot: {}", e);
        e
    })?;

    // Handle graceful shutdown
    let shutdown_signal = tokio::signal::ctrl_c();
    
    tokio::select! {
        result = bot.start() => {
            match result {
                Ok(_) => info!("Bot completed successfully"),
                Err(e) => error!("Bot error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("Shutdown signal received");
            bot.stop().await;
        }
    }

    info!("Polygon Arbitrage Bot shutdown complete");
    Ok(())
}
