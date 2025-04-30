use async_trait::async_trait;
use oc_bots_sdk::api::command::{CommandHandler, SuccessResult};
use oc_bots_sdk::api::definition::*;
use oc_bots_sdk::oc_api::client::Client;
use oc_bots_sdk::types::BotCommandContext;
use oc_bots_sdk_canister::CanisterRuntime;

use std::sync::LazyLock;

use super::common_types::{PriceAlert, ALERTS};
use super::price::Price;

static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(Alert::definition);

pub struct Alert;

#[async_trait]
impl CommandHandler<CanisterRuntime> for Alert {
    fn definition(&self) -> &BotCommandDefinition {
        &DEFINITION
    }

    async fn execute(
        &self,
        client: Client<CanisterRuntime, BotCommandContext>,
    ) -> Result<SuccessResult, String> {
        let coin_id = client
            .context()
            .command
            .arg::<String>("coin")
            .to_lowercase();
        let currency = client
            .context()
            .command
            .arg::<String>("currency")
            .to_lowercase();
        let currency_upper = currency.clone().to_uppercase();
        let target_price = client
            .context()
            .command
            .arg::<String>("price")
            .parse::<f64>()
            .map_err(|_| "Invalid price value. Please enter a valid number.".to_string())?;
        let condition = client
            .context()
            .command
            .arg::<String>("condition")
            .to_lowercase();

        let is_above = match condition.as_str() {
            "above" => true,
            "below" => false,
            _ => return Err("Invalid condition. Must be 'above' or 'below'.".to_string()),
        };

        // Check if the coin exists and get its current price
        let price = Price;
        let price_data = price.fetch_crypto_price(&coin_id, &currency).await?;
        let coin_exists = price_data.prices.contains_key(&price_data.mapped_id);

        if !coin_exists {
            return Err(format!("Coin '{}' not found", coin_id));
        }

        // Create new alert
        let alert = PriceAlert {
            user_id: client.context().command.initiator.to_string(),
            coin_id: price_data.mapped_id.clone(),
            target_price,
            currency,
            is_above,
        };

        // Store the alert
        let mut alerts = ALERTS.lock().await;
        alerts
            .alerts
            .entry(alert.coin_id.clone())
            .or_default()
            .push(alert.clone());

        let condition_text = if is_above { "above" } else { "below" };
        let formatted_msg = format!(
            "✅ Alert set for {} when price goes {} {} {}",
            coin_id.to_uppercase(),
            condition_text,
            target_price,
            currency_upper
        );

        let message = client
            .send_text_message(formatted_msg)
            .with_block_level_markdown(true)
            .execute_then_return_message(|_, _| ());

        Ok(SuccessResult { message })
    }
}

impl Alert {
    fn definition() -> BotCommandDefinition {
        BotCommandDefinition {
            name: "alert".to_string(),
            description: Some("Set a price alert for a cryptocurrency".to_string()),
            placeholder: Some("Setting up price alert...".to_string()),
            params: vec![
                BotCommandParam {
                    name: "coin".to_string(),
                    description: Some(
                        "Cryptocurrency ID or symbol (e.g., bitcoin, BTC)".to_string(),
                    ),
                    placeholder: Some("Enter coin ID or symbol".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 100,
                        choices: Vec::new(),
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "currency".to_string(),
                    description: Some("Currency for the target price".to_string()),
                    placeholder: Some("Choose currency".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 10,
                        choices: vec![
                            BotCommandOptionChoice {
                                name: "USD (US Dollar)".to_string(),
                                value: "usd".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "EUR (Euro)".to_string(),
                                value: "eur".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "KSH (Kenyan Shilling)".to_string(),
                                value: "kes".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "BTC (Bitcoin)".to_string(),
                                value: "btc".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "ETH (Ethereum)".to_string(),
                                value: "eth".to_string(),
                            },
                        ],
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "price".to_string(),
                    description: Some("Target price to trigger the alert".to_string()),
                    placeholder: Some("Enter target price".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 20,
                        choices: Vec::new(),
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "condition".to_string(),
                    description: Some("Alert when price goes above or below target".to_string()),
                    placeholder: Some("Select condition".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 5,
                        choices: vec![
                            BotCommandOptionChoice {
                                name: "Above target price".to_string(),
                                value: "above".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "Below target price".to_string(),
                                value: "below".to_string(),
                            },
                        ],
                        multi_line: false,
                    }),
                },
            ],
            permissions: BotPermissions::from_message_permission(MessagePermission::Text),
            default_role: None,
            direct_messages: Some(true),
        }
    }
}