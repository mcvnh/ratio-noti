use anyhow::Result;
use teloxide::{
    dispatching::dialogue::InMemStorage,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, MaybeInaccessibleMessage, ParseMode},
    utils::command::BotCommands,
};

use crate::{
    binance::BinanceClient,
    config::{Config, RatioPair},
    ratio::RatioCalculator,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Show help message")]
    Help,
    #[command(description = "Show all ratio pairs")]
    Pairs,
    #[command(description = "Get ratio for a specific pair")]
    Ratio,
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub struct BotHandler {
    config: Config,
    calculator: RatioCalculator,
}

impl BotHandler {
    pub fn new(config: Config, calculator: RatioCalculator) -> Self {
        Self { config, calculator }
    }

    pub async fn run(self) -> Result<()> {
        log::info!("Starting interactive Telegram bot...");

        let bot = Bot::new(&self.config.telegram.token);

        let handler = Update::filter_message()
            .branch(
                dptree::entry()
                    .filter_command::<Command>()
                    .endpoint(Self::handle_command),
            )
            .branch(Message::filter_text().endpoint(Self::handle_text));

        let callback_handler = Update::filter_callback_query().endpoint(Self::handle_callback);

        let all_handlers = dptree::entry()
            .branch(handler)
            .branch(callback_handler);

        // Store config and calculator in bot data
        let mut dispatcher = Dispatcher::builder(bot, all_handlers)
            .dependencies(dptree::deps![
                self.config.clone(),
                self.calculator.clone(),
                InMemStorage::<()>::new()
            ])
            .enable_ctrlc_handler()
            .build();

        log::info!("Bot is ready! Send /start to interact");
        dispatcher.dispatch().await;

        Ok(())
    }

    async fn handle_command(
        bot: Bot,
        msg: Message,
        cmd: Command,
        config: Config,
        _calculator: RatioCalculator,
    ) -> HandlerResult {
        match cmd {
            Command::Start => {
                let text = format!(
                    "👋 Welcome to Ratio-Noti Bot!\n\n\
                    I can help you monitor cryptocurrency price ratios from Binance\\.\n\n\
                    *Available Commands:*\n\
                    /pairs \\- View all configured ratio pairs\n\
                    /ratio \\- Get current ratios\n\
                    /help \\- Show this help message\n\n\
                    Click the buttons below or use commands to get started\\!"
                );

                bot.send_message(msg.chat.id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_markup(create_main_keyboard())
                    .await?;
            }
            Command::Help => {
                let text = format!(
                    "🔍 *Ratio\\-Noti Bot Help*\n\n\
                    *Commands:*\n\
                    /start \\- Start the bot\n\
                    /pairs \\- Show all configured pairs\n\
                    /ratio \\- Get current ratios for a pair\n\
                    /help \\- Show this message\n\n\
                    *Features:*\n\
                    ✅ Simple price ratios\n\
                    ✅ Volume\\-based calculations\n\
                    ✅ Real\\-time data from Binance\n\
                    ✅ Interactive pair selection"
                );

                bot.send_message(msg.chat.id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
            Command::Pairs => {
                let text = create_pairs_list(&config);
                bot.send_message(msg.chat.id, text)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
            }
            Command::Ratio => {
                let keyboard = create_pair_selection_keyboard(&config.ratio_pairs);
                bot.send_message(msg.chat.id, "📊 Select a ratio pair:")
                    .reply_markup(keyboard)
                    .await?;
            }
        }

        Ok(())
    }

    async fn handle_text(bot: Bot, msg: Message, _config: Config) -> HandlerResult {
        let text = "Use /start to see available commands or click the buttons below:";

        bot.send_message(msg.chat.id, text)
            .reply_markup(create_main_keyboard())
            .await?;

        Ok(())
    }

    async fn handle_callback(
        bot: Bot,
        q: CallbackQuery,
        config: Config,
        calculator: RatioCalculator,
    ) -> HandlerResult {
        if let Some(data) = &q.data {
            if data.starts_with("ratio:") {
                let pair_name = data.strip_prefix("ratio:").unwrap();
                let pair = config
                    .ratio_pairs
                    .iter()
                    .find(|p| p.name == pair_name)
                    .cloned();

                if let Some(pair) = pair {
                    // Answer the callback query first
                    bot.answer_callback_query(&q.id).await?;

                    // Send "calculating" message
                    if let Some(msg) = q.message {
                        if let Some(chat) = msg.chat() {
                            bot.send_message(chat.id, "⏳ Calculating ratio\\.\\.\\.")
                                .parse_mode(ParseMode::MarkdownV2)
                                .await?;

                            // Calculate ratio
                            match calculator
                                .calculate_simple_ratio(&pair.name, &pair.symbol_a, &pair.symbol_b)
                                .await
                            {
                                Ok(ratio) => {
                                    let text = format!(
                                        "📈 *{}*\n\n\
                                        *Ratio:* `{:.8}`\n\n\
                                        {} \\- ${:.2}\n\
                                        {} \\- ${:.2}\n\n\
                                        _Time: {}_",
                                        escape_markdown(&pair.name),
                                        ratio.ratio,
                                        escape_markdown(&pair.symbol_a),
                                        ratio.price_a,
                                        escape_markdown(&pair.symbol_b),
                                        ratio.price_b,
                                        escape_markdown(&ratio.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                    );

                                    // Check if there's volume configured for detailed analysis
                                    if let Some(volume) = pair.analysis_volume {
                                        bot.send_message(chat.id, text.clone())
                                            .parse_mode(ParseMode::MarkdownV2)
                                            .reply_markup(create_volume_analysis_keyboard(&pair.name, volume))
                                            .await?;
                                    } else {
                                        bot.send_message(chat.id, text)
                                            .parse_mode(ParseMode::MarkdownV2)
                                            .reply_markup(create_back_keyboard())
                                            .await?;
                                    }
                                }
                                Err(e) => {
                                    let error_text = format!(
                                        "❌ Error calculating ratio: {}",
                                        escape_markdown(&e.to_string())
                                    );
                                    bot.send_message(chat.id, error_text)
                                        .parse_mode(ParseMode::MarkdownV2)
                                        .await?;
                                }
                            }
                        }
                    }
                }
            } else if data.starts_with("volume:") {
                let parts: Vec<&str> = data.strip_prefix("volume:").unwrap().split(':').collect();
                if parts.len() == 2 {
                    let pair_name = parts[0];
                    let volume: f64 = parts[1].parse().unwrap_or(1.0);

                    let pair = config
                        .ratio_pairs
                        .iter()
                        .find(|p| p.name == pair_name)
                        .cloned();

                    if let Some(pair) = pair {
                        bot.answer_callback_query(&q.id).await?;

                        if let Some(msg) = q.message {
                            if let Some(chat) = msg.chat() {
                                bot.send_message(chat.id, "⏳ Analyzing order book\\.\\.\\.")
                                    .parse_mode(ParseMode::MarkdownV2)
                                    .await?;

                                match calculator
                                    .calculate_volume_based_ratio(&pair.name, &pair.symbol_a, &pair.symbol_b, volume)
                                    .await
                                {
                                    Ok(ratio) => {
                                        let text = format!(
                                            "📊 *Volume\\-Based Analysis*\n\n\
                                            *Pair:* {}\n\
                                            *Volume:* {}\n\
                                            *Ratio:* `{:.8}`\n\n\
                                            *{}*\n\
                                            Effective Price: ${:.2}\n\
                                            Slippage: {:.3}%\n\n\
                                            *{}*\n\
                                            Effective Price: ${:.2}\n\
                                            Slippage: {:.3}%\n\n\
                                            _Time: {}_",
                                            escape_markdown(&pair.name),
                                            volume,
                                            ratio.ratio,
                                            escape_markdown(&pair.symbol_a),
                                            ratio.effective_price_a,
                                            ratio.slippage_a,
                                            escape_markdown(&pair.symbol_b),
                                            ratio.effective_price_b,
                                            ratio.slippage_b,
                                            escape_markdown(&ratio.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                                        );

                                        bot.send_message(chat.id, text)
                                            .parse_mode(ParseMode::MarkdownV2)
                                            .reply_markup(create_back_keyboard())
                                            .await?;
                                    }
                                    Err(e) => {
                                        let error_text = format!(
                                            "❌ Error analyzing volume: {}",
                                            escape_markdown(&e.to_string())
                                        );
                                        bot.send_message(chat.id, error_text)
                                            .parse_mode(ParseMode::MarkdownV2)
                                            .await?;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if data == "back_to_pairs" {
                bot.answer_callback_query(&q.id).await?;

                if let Some(msg) = q.message {
                    if let Some(chat) = msg.chat() {
                        let keyboard = create_pair_selection_keyboard(&config.ratio_pairs);
                        bot.send_message(chat.id, "📊 Select a ratio pair:")
                            .reply_markup(keyboard)
                            .await?;
                    }
                }
            } else if data == "main_menu" {
                bot.answer_callback_query(&q.id).await?;

                if let Some(msg) = q.message {
                    if let Some(chat) = msg.chat() {
                        bot.send_message(chat.id, "Main menu:")
                            .reply_markup(create_main_keyboard())
                            .await?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn create_main_keyboard() -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![InlineKeyboardButton::callback("📊 Get Ratios", "main:ratios")],
        vec![InlineKeyboardButton::callback("📋 View Pairs", "main:pairs")],
    ];

    InlineKeyboardMarkup::new(buttons)
}

fn create_pair_selection_keyboard(pairs: &[RatioPair]) -> InlineKeyboardMarkup {
    let mut buttons: Vec<Vec<InlineKeyboardButton>> = pairs
        .iter()
        .map(|pair| {
            vec![InlineKeyboardButton::callback(
                &pair.name,
                format!("ratio:{}", pair.name),
            )]
        })
        .collect();

    buttons.push(vec![InlineKeyboardButton::callback("« Back", "main_menu")]);

    InlineKeyboardMarkup::new(buttons)
}

fn create_volume_analysis_keyboard(pair_name: &str, volume: f64) -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![InlineKeyboardButton::callback(
            format!("📊 Volume Analysis ({})", volume),
            format!("volume:{}:{}", pair_name, volume),
        )],
        vec![InlineKeyboardButton::callback("« Back to Pairs", "back_to_pairs")],
    ];

    InlineKeyboardMarkup::new(buttons)
}

fn create_back_keyboard() -> InlineKeyboardMarkup {
    let buttons = vec![
        vec![InlineKeyboardButton::callback("« Back to Pairs", "back_to_pairs")],
        vec![InlineKeyboardButton::callback("« Main Menu", "main_menu")],
    ];

    InlineKeyboardMarkup::new(buttons)
}

fn create_pairs_list(config: &Config) -> String {
    let mut text = String::from("📋 *Configured Ratio Pairs*\n\n");

    for (i, pair) in config.ratio_pairs.iter().enumerate() {
        text.push_str(&format!(
            "{}\\. *{}*\n   {} / {}\n",
            i + 1,
            escape_markdown(&pair.name),
            escape_markdown(&pair.symbol_a),
            escape_markdown(&pair.symbol_b)
        ));

        if let Some(vol) = pair.analysis_volume {
            text.push_str(&format!("   Volume: {}\n", vol));
        }
        text.push('\n');
    }

    text
}

fn escape_markdown(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '=' | '|' | '{' | '}' | '.' | '!' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect()
}

impl Clone for RatioCalculator {
    fn clone(&self) -> Self {
        Self::new(BinanceClient::new())
    }
}
