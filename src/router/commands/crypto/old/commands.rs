use crate::state;
use crate::router::commands::echo::Echo;
use crate::router::commands::crypto::{
    Alert, Market, Price, Trending, SetTrade
};
use oc_bots_sdk::api::command::CommandHandlerRegistry;
use oc_bots_sdk::api::definition::BotCommandDefinition;
use oc_bots_sdk_canister::env::now;
use oc_bots_sdk_canister::http_command_handler;
use oc_bots_sdk_canister::CanisterRuntime;
use oc_bots_sdk_canister::OPENCHAT_CLIENT_FACTORY;
use oc_bots_sdk_canister::{HttpRequest, HttpResponse};
use std::sync::LazyLock;

static COMMANDS: LazyLock<CommandHandlerRegistry<CanisterRuntime>> =
    LazyLock::new(|| CommandHandlerRegistry::new(OPENCHAT_CLIENT_FACTORY.clone())
        .register(Echo)
        .register(Price)
        .register(Market)
        .register(Trending)
        .register(Alert)
        .register(SetTrade)
    );

pub fn definitions() -> Vec<BotCommandDefinition> {
    COMMANDS.definitions()
}

pub async fn execute(request: HttpRequest) -> HttpResponse {
    let public_key = state::read(|state| state.oc_public_key().to_string());
    let now = now();
    http_command_handler::execute(request, &COMMANDS, &public_key, now).await
}
