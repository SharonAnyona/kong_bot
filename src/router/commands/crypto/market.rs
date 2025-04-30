use async_trait::async_trait;
use oc_bots_sdk::api::command::{CommandHandler, SuccessResult};
use oc_bots_sdk::api::definition::*;
use oc_bots_sdk::oc_api::client::Client;
use oc_bots_sdk::types::BotCommandContext;
use oc_bots_sdk_canister::CanisterRuntime;
use std::sync::LazyLock;
use super::common_types::MarketData;
use oc_bots_sdk_canister::{HttpRequest, HttpResponse};
use ic_cdk::api::management_canister::http_request::{
    http_request, HttpRequest, HttpResponse, TransformArgs, TransformContext,
};
use ic_cdk::api::management_canister::http_request::HttpMethod;

pub struct Market;

static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(Market::definition);

#[async_trait]
impl CommandHandler<CanisterRuntime> for Market {
    fn definition(&self) -> &BotCommandDefinition {
        &DEFINITION
    }

    async fn execute(
        &self,
        client: Client<CanisterRuntime, BotCommandContext>,
    ) -> Result<SuccessResult, String> {
        let count_str = client.context().command.arg::<String>("count");
        let count: u8 = count_str
            .parse()
            .map_err(|_| "Invalid count. Please select 5, 10, or 25.")?;

        let currency = client
            .context()
            .command
            .arg::<String>("currency")
            .to_lowercase();

        let market_data = self.fetch_market_data(count, &currency).await?;

        let mut message = format!("📊 **Top {} Cryptocurrencies**\n\n", count);
        for (i, coin) in market_data.iter().enumerate() {
            let price = coin.current_price.get(&currency).unwrap_or(&0.0);
            let change = coin.price_change_percentage_24h.unwrap_or(0.0);
            let emoji = if change >= 0.0 { "🟢" } else { "🔴" };

            message.push_str(&format!(
                "{}. **{}** ({})\nPrice: {} {}\n24h Change: {}{:.2}%\n\n",
                i + 1,
                coin.name,
                coin.symbol.to_uppercase(),
                price,
                currency.to_uppercase(),
                emoji,
                change
            ));
        }

        let message = client
            .send_text_message(message)
            .with_block_level_markdown(true)
            .execute_then_return_message(|_, _| ());

        Ok(SuccessResult { message })
    }
}

impl Market {
    async fn fetch_market_data(&self, count: u8, currency: &str) -> Result<Vec<MarketData>, String> {
        let url = format!(
            "https://api.coingecko.com/api/v3/coins/markets?vs_currency={}&order=market_cap_desc&per_page={}&page=1&sparkline=false",
            currency, count
        );

        let response = self.make_http_request(&url).await?;

        if response.status_code != 200 {
            return Err(format!("API returned status: {}", response.status_code));
        }

        let parsed: Vec<MarketData> = serde_json::from_slice(&response.body)
            .map_err(|e| format!("Failed to parse market data: {}", e))?;

        Ok(parsed)
    }
    async fn make_http_request(&self, url: &str) -> Result<HttpResponse, String> {
        let request = HttpRequest {
            url: url.to_string(),
            method: HttpMethod::GET,
            headers: vec![
                ("Accept".to_string(), "application/json".to_string()),
                ("User-Agent".to_string(), "KongBot/1.0".to_string()),
            ],
            body: vec![],
            max_response_bytes: Some(2 * 1024 * 1024),
            transform: None,
        };
    
        http_request(request, 50_000_000_000)
            .await
            .map(|(res,)| res)
            .map_err(|(code, msg)| format!("HTTP request failed: {:?} - {}", code, msg))
    }
    
    fn definition() -> BotCommandDefinition {
        BotCommandDefinition {
            name: "market".into(),
            description: Some("Get top cryptocurrencies by market cap".into()),
            placeholder: Some("Fetching market data...".into()),
            params: vec![
                BotCommandParam {
                    name: "count".into(),
                    description: Some("Number of coins (5, 10, 25)".into()),
                    placeholder: Some("Choose number of coins".into()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 1,
                        max_length: 2,
                        choices: vec![
                            ("5 coins", "5"),
                            ("10 coins", "10"),
                            ("25 coins", "25"),
                        ]
                        .into_iter()
                        .map(|(n, v)| BotCommandOptionChoice {
                            name: n.into(),
                            value: v.into(),
                        })
                        .collect(),
                        multi_line: false,
                    }),
                },
                BotCommandParam {
                    name: "currency".into(),
                    description: Some("Currency to display prices in".into()),
                    placeholder: Some("Select currency".into()),
                    required: true,
                    param_type: BotCommandParamType::StringParam(StringParam {
                        min_length: 3,
                        max_length: 3,
                        choices: vec![
                            ("USD (US Dollar)", "usd"),
                            ("EUR (Euro)", "eur"),
                            ("KSH (Kenyan Shilling)", "kes"),
                            ("BTC (Bitcoin)", "btc"),
                            ("ETH (Ethereum)", "eth"),
                        ]
                        .into_iter()
                        .map(|(n, v)| BotCommandOptionChoice {
                            name: n.into(),
                            value: v.into(),
                        })
                        .collect(),
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
