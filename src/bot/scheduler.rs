use anyhow::Result;
use std::time::Duration;
use tokio::{
    sync::{broadcast, mpsc},
    time::{interval, Instant},
};
use tracing::{debug, error, info, warn};

use crate::bot::ArbitrageBot;

#[derive(Debug, Clone)]
pub enum BotCommand {
    Start,
    Stop,
    Pause,
    Resume,
    UpdateConfig,
    GetStats,
}

#[derive(Debug, Clone)]
pub enum BotEvent {
    Started,
    Stopped,
    Paused,
    Resumed,
    OpportunityFound { count: usize, total_profit: String },
    Error { message: String },
    Stats { stats: String },
}

pub struct BotScheduler {
    command_sender: mpsc::UnboundedSender<BotCommand>,
    event_receiver: broadcast::Receiver<BotEvent>,
    _event_sender: broadcast::Sender<BotEvent>, // Keep sender alive
}

impl BotScheduler {
    pub fn new() -> Self {
        let (command_sender, command_receiver) = mpsc::unbounded_channel();
        let (event_sender, event_receiver) = broadcast::channel(100);

        // Spawn the scheduler task
        let event_sender_clone = event_sender.clone();
        tokio::spawn(async move {
            Self::run_scheduler(command_receiver, event_sender_clone).await;
        });

        Self {
            command_sender,
            event_receiver,
            _event_sender: event_sender,
        }
    }

    pub fn send_command(&self, command: BotCommand) -> Result<()> {
        self.command_sender.send(command)
            .map_err(|e| anyhow::anyhow!("Failed to send command: {}", e))?;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<BotEvent> {
        self.event_receiver.recv().await
            .map_err(|e| anyhow::anyhow!("Failed to receive event: {}", e))
    }

    async fn run_scheduler(
        mut command_receiver: mpsc::UnboundedReceiver<BotCommand>,
        event_sender: broadcast::Sender<BotEvent>,
    ) {
        info!("Bot scheduler started");
        
        let mut bot_state = BotState::Stopped;
        let mut last_heartbeat = Instant::now();
        let mut heartbeat_interval = interval(Duration::from_secs(60));

        loop {
            tokio::select! {
                // Handle incoming commands
                command = command_receiver.recv() => {
                    match command {
                        Some(cmd) => {
                            debug!("Received command: {:?}", cmd);
                            Self::handle_command(cmd, &mut bot_state, &event_sender).await;
                        }
                        None => {
                            warn!("Command channel closed, stopping scheduler");
                            break;
                        }
                    }
                }
                
                // Periodic heartbeat
                _ = heartbeat_interval.tick() => {
                    let now = Instant::now();
                    if now.duration_since(last_heartbeat) > Duration::from_secs(300) {
                        warn!("Bot heartbeat timeout detected");
                        if matches!(bot_state, BotState::Running) {
                            let _ = event_sender.send(BotEvent::Error {
                                message: "Bot heartbeat timeout".to_string(),
                            });
                        }
                    }
                    last_heartbeat = now;
                }
            }
        }

        info!("Bot scheduler stopped");
    }

    async fn handle_command(
        command: BotCommand,
        bot_state: &mut BotState,
        event_sender: &broadcast::Sender<BotEvent>,
    ) {
        match command {
            BotCommand::Start => {
                if matches!(bot_state, BotState::Stopped | BotState::Paused) {
                    *bot_state = BotState::Running;
                    let _ = event_sender.send(BotEvent::Started);
                    info!("Bot started");
                } else {
                    warn!("Cannot start bot - already running");
                }
            }
            
            BotCommand::Stop => {
                if !matches!(bot_state, BotState::Stopped) {
                    *bot_state = BotState::Stopped;
                    let _ = event_sender.send(BotEvent::Stopped);
                    info!("Bot stopped");
                }
            }
            
            BotCommand::Pause => {
                if matches!(bot_state, BotState::Running) {
                    *bot_state = BotState::Paused;
                    let _ = event_sender.send(BotEvent::Paused);
                    info!("Bot paused");
                }
            }
            
            BotCommand::Resume => {
                if matches!(bot_state, BotState::Paused) {
                    *bot_state = BotState::Running;
                    let _ = event_sender.send(BotEvent::Resumed);
                    info!("Bot resumed");
                }
            }
            
            BotCommand::UpdateConfig => {
                info!("Config update requested");
                // In a real implementation, this would reload configuration
            }
            
            BotCommand::GetStats => {
                let stats_message = format!("Bot State: {:?}", bot_state);
                let _ = event_sender.send(BotEvent::Stats {
                    stats: stats_message,
                });
            }
        }
    }
}

#[derive(Debug, Clone)]
enum BotState {
    Stopped,
    Running,
    Paused,
}

impl Default for BotScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_scheduler_commands() {
        let mut scheduler = BotScheduler::new();

        // Test start command
        scheduler.send_command(BotCommand::Start).unwrap();
        let event = timeout(Duration::from_secs(1), scheduler.next_event()).await.unwrap().unwrap();
        assert!(matches!(event, BotEvent::Started));

        // Test stop command
        scheduler.send_command(BotCommand::Stop).unwrap();
        let event = timeout(Duration::from_secs(1), scheduler.next_event()).await.unwrap().unwrap();
        assert!(matches!(event, BotEvent::Stopped));
    }
}
