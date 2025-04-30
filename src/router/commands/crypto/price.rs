use async_trait::async_trait;
use oc_bots_sdk::api::command::{CommandHandler, SuccessResult};
use oc_bots_sdk::api::definition::*;
use oc_bots_sdk::oc_api::client::Client;
use oc_bots_sdk::types::BotCommandContext;
use oc_bots_sdk_canister::CanisterRuntime;
use std::sync::LazyLock;
use super::common_types::PriceResponse;
use ic_cdk::api::management_canister::http_request::*;

// Command definition
static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(Price::definition);

// Price command handler
pub struct Price;

#[async_trait]
impl CommandHandler<CanisterRuntime> for Price {
    fn definition(&self) -> &BotCommandDefinition {
        &DEFINITION
    }

    async fn execute(
        &self,
        client: Client<CanisterRuntime, BotCommandContext>,
    ) -> Result<SuccessResult, String> {
        let coin_id = client.context().command.arg::<String>("coin").to_lowercase();
        let currency = client.context().command.arg::<String>("currency").to_lowercase();

        // Get price from Coingecko
        match self.fetch_crypto_price(&coin_id, &currency).await {
            Ok(price_data) => {
                // When using ID mapping, we need to look for the actual_coin_id in the response
                let formatted_msg = if price_data.mapped_id != coin_id {
                    match price_data.prices.get(&price_data.mapped_id) {
                        Some(prices) => match prices.values.get(&currency) {
                            Some(price) => {
                                format!(
                                    "💰 **{} ({})**: {} {}",
                                    coin_id.to_uppercase(),
                                    price_data.mapped_id,
                                    price,
                                    currency.to_uppercase()
                                )
                            }
                            None => format!("Currency {} not found for {}", currency, coin_id),
                        },
                        None => format!("Coin {} not found", coin_id),
                    }
                } else {
                    match price_data.prices.get(&coin_id) {
                        Some(prices) => match prices.values.get(&currency) {
                            Some(price) => {
                                format!(
                                    "💰 **{} ({})**: {} {}",
                                    coin_id.to_uppercase(),
                                    coin_id,
                                    price,
                                    currency.to_uppercase()
                                )
                            }
                            None => format!("Currency {} not found for {}", currency, coin_id),
                        },
                        None => format!("Coin {} not found", coin_id),
                    }
                };

                let message = client
                    .send_text_message(formatted_msg)
                    .with_block_level_markdown(true)
                    .execute_then_return_message(|_, _| ());

                Ok(SuccessResult { message })
            }
            Err(e) => {
                let error_msg = format!("Error fetching price data: {}", e);
                let message = client
                    .send_text_message(error_msg)
                    .with_block_level_markdown(true)
                    .execute_then_return_message(|_, _| ());

                Ok(SuccessResult { message })
            }
        }
    }
}

impl Price {
    // Updated API call to fetch crypto price using IC's HTTP request
    pub async fn fetch_crypto_price(
        &self,
        coin_id: &str,
        currency: &str,
    ) -> Result<PriceResponse, String> {
        // First try to see if we need to map a symbol to an ID
        let mut actual_coin_id = coin_id.to_string();

        // If the coin ID looks like a symbol (short, all uppercase) or just short
        // We're removing the uppercase check to catch symbols like "btc" also
        if coin_id.len() <= 5 {
            // Try to get the actual coin ID from the search endpoint
            let search_url = format!("https://api.coingecko.com/api/v3/search?query={}", coin_id);
            
            match self.make_http_request(&search_url).await {
                Ok(response) => {
                    if response.status == 200 {
                        // Parse the search result
                        #[derive(Debug, serde::Deserialize)]
                        struct SearchResult {
                            coins: Vec<SearchCoin>,
                        }
                        
                        #[derive(Debug, serde::Deserialize)]
                        struct SearchCoin {
                            id: String,
                            symbol: String,
                        }
                        
                        if let Ok(search_data) = serde_json::from_slice::<SearchResult>(&response.body) {
                            // Find the coin with the exact symbol match
                            for coin in search_data.coins {
                                if coin.symbol.to_lowercase() == coin_id.to_lowercase() {
                                    actual_coin_id = coin.id;
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(_) => { /* Continue with original id if search fails */ }
            }
        }

        // Now fetch the price data with the actual ID
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies={}",
            actual_coin_id, currency
        );
        
        let response = match self.make_http_request(&url).await {
            Ok(res) => res,
            Err(e) => return Err(format!("Failed to fetch price data: {}", e)),
        };

        if response.status != 200 {
            return Err(format!("API request failed with status: {}", response.status));
        }

        let mut price_data = match serde_json::from_slice::<PriceResponse>(&response.body) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to parse price data: {}", e)),
        };

        // Set the mapped_id field
        price_data.mapped_id = actual_coin_id;
        
        Ok(price_data)
    }

    // Helper method to make HTTP requests using IC management canister
    async fn make_http_request(&self, url: &str) -> Result<HttpResponse, String> {
        let request_headers = vec![
            ("Accept".to_string(), "application/json".to_string()),
            ("User-Agent".to_string(), "KongBot/1.0".to_string()),
        ];

        let request = ic_cdk::api::management_canister::http_request::HttpRequest {
            url: url.to_string(),
            method: "GET".to_string(),
            body: vec![],
            headers: request_headers,
            max_response_bytes: Some(2 * 1024 * 1024), // 2MB max response
        };

        match ic_cdk::api::management_canister::http_request::http_request(
            request,
            50_000_000_000, // 50B cycles for the HTTP request
        ).await {
            Ok((response,)) => Ok(response),
            Err((code, msg)) => Err(format!("HTTP request failed with code {:?}: {}", code, msg)),
        }
    }

    fn definition() -> BotCommandDefinition {
        BotCommandDefinition {
            name: "price".to_string(),
            description: Some("Check the current price of a cryptocurrency".to_string()),
            placeholder: Some("Fetching price information...".to_string()),
            params: vec![
                BotCommandParam {
                    name: "coin".to_string(),
                    description: Some(
                        "Cryptocurrency ID or symbol (e.g., bitcoin, BTC, ethereum, ETH)".to_string(),
                    ),
                    placeholder: Some("Enter coin ID or symbol (e.g., bitcoin, BTC)".to_string()),
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
                    description: Some("Currency to display price in (e.g., usd, eur, ksh)".to_string()),
                    placeholder: Some("Enter currency (default: usd)".to_string()),
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
            ],
            permissions: BotPermissions::from_message_permission(MessagePermission::Text),
            default_role: None,
            direct_messages: Some(true),
        }
    }
}
