use async_trait::async_trait;
use oc_bots_sdk::api::command::{CommandHandler, SuccessResult};
use oc_bots_sdk::api::definition::*;
use oc_bots_sdk::oc_api::client::Client;
use oc_bots_sdk::types::BotCommandContext;
use oc_bots_sdk_canister::CanisterRuntime;
use std::sync::LazyLock;
use super::common_types::{TrendingResponse, TrendingResult};
use oc_bots_sdk_canister::{HttpRequest, HttpResponse};

// Command definition
static DEFINITION: LazyLock<BotCommandDefinition> = LazyLock::new(Trending::definition);

// Trending command handler
pub struct Trending;

#[async_trait]
impl CommandHandler<CanisterRuntime> for Trending {
    fn definition(&self) -> &BotCommandDefinition {
        &DEFINITION
    }

    async fn execute(
        &self,
        client: Client<CanisterRuntime, BotCommandContext>,
    ) -> Result<SuccessResult, String> {
        // Get trending data from Coingecko
        match self.fetch_trending_coins().await {
            Ok(trending_result) => {
                let mut formatted_msg = "🔥 **Trending Cryptocurrencies**\n\n".to_string();

                // Use trending data and USD prices
                for (i, coin) in trending_result.trending_data.coins.iter().enumerate() {
                    let rank = coin.item.market_cap_rank.map_or("N/A".to_string(), |r| r.to_string());
                    let usd_price = self.get_usd_price(&coin.item.id, &trending_result.prices_data);

                    formatted_msg.push_str(&format!(
                        "{}. **{}** ({})\n Market Cap Rank: {}\n BTC Price: {:.8}\n USD Price: {}\n\n",
                        i + 1,
                        coin.item.name,
                        coin.item.symbol.to_uppercase(),
                        rank,
                        coin.item.price_btc,
                        usd_price
                    ));
                }

                let message = client
                    .send_text_message(formatted_msg)
                    .with_block_level_markdown(true)
                    .execute_then_return_message(|_, _| ());

                Ok(SuccessResult { message })
            }
            Err(e) => {
                let error_msg = format!("Error fetching trending data: {}", e);
                let message = client
                    .send_text_message(error_msg)
                    .with_block_level_markdown(true)
                    .execute_then_return_message(|_, _| ());

                Ok(SuccessResult { message })
            }
        }
    }
}

impl Trending {
    async fn fetch_trending_coins(&self) -> Result<TrendingResult, String> {
        // First get the trending coins list
        let trending_url = "https://api.coingecko.com/api/v3/search/trending";
        let trending_response = match self.make_http_request(trending_url).await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to fetch trending data: {}", e)),
        };

        if trending_response.status_code != 200 {
            return Err(format!("API request failed with status: {}", trending_response.status));
        }

        let trending_data = match serde_json::from_slice::<TrendingResponse>(&trending_response.body) {
            Ok(data) => data,
            Err(e) => return Err(format!("Failed to parse trending data: {}", e)),
        };

        // For each trending coin, we want to fetch USD price
        // But to respect the rate limits, we'll collect the ids first
        let coin_ids: Vec<String> = trending_data.coins.iter().map(|coin| coin.item.id.clone()).collect();

        // Now fetch the USD prices for these coins
        let ids_param = coin_ids.join(",");
        let prices_url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
            ids_param
        );

        let prices_response = match self.make_http_request(&prices_url).await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to fetch USD prices: {}", e)),
        };

        let prices_data = if prices_response.status_code == 200 {
            match serde_json::from_slice::<std::collections::HashMap<String, std::collections::HashMap<String, f64>>>(&prices_response.body) {
                Ok(data) => Some(data),
                Err(e) => return Err(format!("Failed to parse USD prices: {}", e)),
            }
        } else {
            None
        };

        // Return both the trending data and price data
        Ok(TrendingResult {
            trending_data,
            prices_data,
        })
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

    // Helper method to get USD price for a coin id
    fn get_usd_price(
        &self,
        coin_id: &str,
        prices: &Option<std::collections::HashMap<String, std::collections::HashMap<String, f64>>>,
    ) -> String {
        match prices {
            Some(price_map) => match price_map.get(coin_id) {
                Some(currencies) => match currencies.get("usd") {
                    Some(price) => format!("${:.2}", price),
                    None => "N/A".to_string(),
                },
                None => "N/A".to_string(),
            },
            None => "N/A".to_string(),
        }
    }

    fn definition() -> BotCommandDefinition {
        BotCommandDefinition {
            name: "trending".to_string(),
            description: Some("Show trending cryptocurrencies".to_string()),
            placeholder: Some("Fetching trending coins...".to_string()),
            params: vec![],
            permissions: BotPermissions::from_message_permission(MessagePermission::Text),
            default_role: None,
            direct_messages: Some(true),
        }
    }
}
