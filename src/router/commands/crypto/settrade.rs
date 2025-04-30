use async_trait::async_trait;
use lazy_static::lazy_static;
use oc_bots_sdk::api::command::{CommandHandler, SuccessResult};
use oc_bots_sdk::api::definition::*;
use oc_bots_sdk::oc_api::client::Client;
use oc_bots_sdk::types::{BotCommandContext, BotCommandScope, BotPermissions, Chat, MessagePermission};
use oc_bots_sdk_canister::CanisterRuntime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::Mutex;
use super::icrc1_transfer::icrc1_transfer_token;
use super::icrc2_approve::icrc2_approve_token;

// Command definition
static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(SetTrade::definition);

// Define a global storage for trade orders
lazy_static! {
    pub static ref TRADE_ORDERS: Arc<Mutex<TradeOrderStorage>> = Arc::new(Mutex::new(TradeOrderStorage::new()));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOrder {
    pub user_id: String,
    pub coin_id: String,
    pub target_price: f64,
    pub amount: f64,
    pub wallet: String,
    pub is_buy: bool,
    pub created_at: String, // ISO 8601 formatted date string
    pub token_canister_id: String, // Token canister ID
    pub approved: bool, // Whether the user has approved the bot to spend tokens
}

#[derive(Debug, Clone, Default)]
pub struct TradeOrderStorage {
    // Map of coin_id -> list of trade orders
    pub orders: HashMap<String, Vec<TradeOrder>>,
}

impl TradeOrderStorage {
    pub fn new() -> Self {
        Self {
            orders: HashMap::new(),
        }
    }

    // Add a trade order
    pub fn add_order(&mut self, order: TradeOrder) {
        let orders = self.orders.entry(order.coin_id.clone()).or_insert_with(Vec::new);
        orders.push(order);
    }

    // Update approval status for an order
    pub fn update_approval(&mut self, user_id: &str, coin_id: &str, approved: bool) {
        if let Some(orders) = self.orders.get_mut(coin_id) {
            for order in orders {
                if order.user_id == user_id {
                    order.approved = approved;
                }
            }
        }
    }

    // Find a matching order
    pub async fn match_orders(&mut self, order: &TradeOrder) -> Option<TradeOrder> {
        if let Some(orders) = self.orders.get_mut(&order.coin_id) {
            // Find a matching order (opposite buy/sell, compatible price)
            let matching_idx = orders.iter().position(|o| {
                o.user_id != order.user_id &&
                o.is_buy != order.is_buy &&
                (o.target_price >= order.target_price && order.is_buy ||
                 o.target_price <= order.target_price && !order.is_buy)
            });

            // If found, remove and return it
            if let Some(idx) = matching_idx {
                return Some(orders.remove(idx));
            }
        }
        None
    }
}

pub struct SetTrade;

#[async_trait]
impl CommandHandler<CanisterRuntime> for SetTrade {
    fn definition(&self) -> &BotCommandDefinition {
        &DEFINITION
    }

    async fn execute(
        &self,
        client: Client<CanisterRuntime, BotCommandContext>,
    ) -> Result<SuccessResult, String> {
        let context = client.context();
        let command = &context.command;
        
        let user_id = match &context.scope {
            BotCommandScope::Chat(chat_details) => {
                match &chat_details.chat {
                    Chat::Direct(canister_id) => format!("user_{}", canister_id),
                    Chat::Group(canister_id) => format!("group_user_{}", canister_id),
                    Chat::Channel(community_id, channel_id) =>
                        format!("channel_user_{}_{}", community_id, channel_id),
                }
            }
            BotCommandScope::Community(community_details) => {
                format!("community_user_{}", community_details.community_id)
            }
        };
        
        let action = command.arg::<String>("action").to_lowercase();
        if action != "buy" && action != "sell" {
            return send_error(client, "Action must be either 'buy' or 'sell'").await;
        }
        
        let coin = command.arg::<String>("coin").to_lowercase();
        let token_canister_id = match coin.as_str() {
            "icp" => "ryjl3-tyaaa-aaaaa-aaaba-cai",
            "inwt" => "zfcdd-tqaaa-aaaaq-aaaga-cai",
            _ => return send_error(client, "Unsupported token").await,
        }.to_string();
        
        let price = command.arg::<String>("price").parse::<f64>().ok()
            .filter(|p| *p > 0.0)
            .ok_or_else(|| "Invalid price".to_string())?;
            
        let amount = command.arg::<String>("amount").parse::<f64>().ok()
            .filter(|a| *a > 0.0)
            .ok_or_else(|| "Invalid amount".to_string())?;
            
        // Make wallet a required parameter
        let wallet = command.arg::<String>("wallet");
        if wallet.trim().is_empty() {
            return send_error(client, "Wallet address is required").await;
        }
        
        let is_buy = action == "buy";
        
        let new_order = TradeOrder {
            user_id: user_id.clone(),
            coin_id: coin.clone(),
            token_canister_id: token_canister_id.clone(),
            target_price: price,
            amount,
            wallet: wallet.clone(),
            is_buy,
            created_at: ic_cdk::api::time().to_string(), // Use IC time
            approved: false,
        };
        
        let mut orders = TRADE_ORDERS.lock().await;
        
        // Match an opposite trade order
        if let Some(matched_order) = orders.match_orders(&new_order).await {
            ic_cdk::println!("Matched order between {} and {}", new_order.user_id, matched_order.user_id);
            
            let from = if new_order.is_buy { matched_order.wallet.clone() } else { new_order.wallet.clone() };
            let to = if new_order.is_buy { new_order.wallet.clone() } else { matched_order.wallet.clone() };
            
            // Approve bot to spend tokens if not already done
            if !matched_order.approved {
                match icrc2_approve_token(
                    &token_canister_id,
                    &matched_order.wallet,
                    &wallet, // Use the user's wallet address
                    matched_order.amount
                ).await {
                    Ok(_) => {
                        orders.update_approval(&matched_order.user_id, &matched_order.coin_id, true);
                    },
                    Err(e) => {
                        return send_error(client, &format!("Failed to approve token: {}", e)).await;
                    }
                }
            }
            
            // Transfer token between users
            match icrc1_transfer_token(
                &token_canister_id,
                &from,
                &to,
                matched_order.amount
            ).await {
                Ok(_) => {},
                Err(e) => {
                    return send_error(client, &format!("Failed to transfer token: {}", e)).await;
                }
            }
            
            let message = format!(
                "✅ Trade executed: {} {} {} at ${} from {} to {}",
                if new_order.is_buy { "BUY" } else { "SELL" },
                matched_order.amount,
                coin.to_uppercase(),
                price,
                from,
                to
            );
            
            return send_success(client, &message).await;
        }
        
        // No match found – store for future
        orders.add_order(new_order.clone());
        
        let response = format!(
            "✅ Trade order set: {} {} {} at ${} with wallet {}.\nWaiting for a match...",
            if is_buy { "BUY" } else { "SELL" },
            amount,
            coin.to_uppercase(),
            price,
            wallet
        );
        
        send_success(client, &response).await
    }
}

async fn send_error(client: Client<CanisterRuntime, BotCommandContext>, msg: &str) -> Result<SuccessResult, String> {
    let message = client.send_text_message(msg.to_string())
        .execute_then_return_message(|_, _| ());
    Ok(SuccessResult { message })
}

async fn send_success(client: Client<CanisterRuntime, BotCommandContext>, msg: &str) -> Result<SuccessResult, String> {
    let message = client.send_text_message(msg.to_string())
        .with_block_level_markdown(true)
        .execute_then_return_message(|_, _| ());
    Ok(SuccessResult { message })
}

impl SetTrade {
    fn definition() -> BotCommandDefinition {
        BotCommandDefinition {
            name: "settrade".to_string(),
            description: Some("Set a trade order (buy or sell) for a cryptocurrency at a target price".to_string()),
            placeholder: Some("Setting up your trade order...".to_string()),
            params: vec![
                BotCommandParam {
                    name: "action".to_string(),
                    description: Some("Buy or sell action".to_string()),
                    placeholder: Some("buy or sell".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 4,
                        choices: vec![
                            BotCommandOptionChoice {
                                name: "Buy".to_string(),
                                value: "buy".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "Sell".to_string(),
                                value: "sell".to_string(),
                            },
                        ],
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "coin".to_string(),
                    description: Some("Cryptocurrency ID (e.g., bitcoin, ethereum, icp)".to_string()),
                    placeholder: Some("Enter coin ID (e.g., bitcoin, icp, inwt)".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 100,
                        choices: vec![
                            BotCommandOptionChoice {
                                name: "ICP".to_string(),
                                value: "icp".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "INWT".to_string(),
                                value: "inwt".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "Bitcoin".to_string(),
                                value: "bitcoin".to_string(),
                            },
                            BotCommandOptionChoice {
                                name: "Ethereum".to_string(),
                                value: "ethereum".to_string(),
                            },
                        ],
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "price".to_string(),
                    description: Some("Target price in USD".to_string()),
                    placeholder: Some("30000".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 20,
                        choices: Vec::new(),
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "amount".to_string(),
                    description: Some("Amount to buy or sell".to_string()),
                    placeholder: Some("0.1".to_string()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 20,
                        choices: Vec::new(),
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "wallet".to_string(),
                    description: Some("Your wallet address (required)".to_string()),
                    placeholder: Some("Enter your wallet address (e.g., principal ID)".to_string()),
                    required: true, // Changed to required
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1, // Require at least one character
                        max_length: 100,
                        choices: Vec::new(),
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
