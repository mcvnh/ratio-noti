use anyhow::{Context, Result};
use teloxide::prelude::*;
use teloxide::types::ChatId;

pub struct TelegramNotifier {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramNotifier {
    pub fn new(token: &str, user_id: i64) -> Self {
        Self {
            bot: Bot::new(token),
            chat_id: ChatId(user_id),
        }
    }

    /// Send a text message to the configured user
    pub async fn send_message(&self, message: &str) -> Result<()> {
        self.bot
            .send_message(self.chat_id, message)
            .await
            .context("Failed to send Telegram message")?;

        Ok(())
    }

    /// Send a formatted ratio alert message
    pub async fn send_ratio_alert(&self, pair_name: &str, ratio: f64, change_pct: f64, time_window: &str) -> Result<()> {
        let emoji = if change_pct > 0.0 { "📈" } else { "📉" };
        let message = format!(
            "{} *Ratio Alert: {}*\n\n\
            Current Ratio: `{:.8}`\n\
            Change: `{:+.2}%` in {}\n\
            Time: {}",
            emoji,
            pair_name,
            ratio,
            change_pct,
            time_window,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await
            .context("Failed to send ratio alert")?;

        Ok(())
    }

    /// Send a periodic ratio update
    pub async fn send_periodic_update(&self, updates: &[String]) -> Result<()> {
        let message = format!(
            "📊 *Periodic Ratio Update*\n\n{}\n\n_Time: {}_",
            updates.join("\n\n"),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await
            .context("Failed to send periodic update")?;

        Ok(())
    }

    /// Send a slippage analysis message
    pub async fn send_slippage_analysis(&self, analysis: &str) -> Result<()> {
        let message = format!(
            "🔍 *Slippage Analysis*\n\n```\n{}\n```",
            analysis
        );

        self.bot
            .send_message(self.chat_id, message)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await
            .context("Failed to send slippage analysis")?;

        Ok(())
    }

    /// Test the connection by sending a test message
    pub async fn test_connection(&self) -> Result<()> {
        let message = "✅ Ratio-Noti bot is connected and ready!";

        self.bot
            .send_message(self.chat_id, message)
            .await
            .context("Failed to send test message")?;

        Ok(())
    }
}
