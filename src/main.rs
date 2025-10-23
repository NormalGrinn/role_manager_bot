use std::env;

use dotenvy::dotenv;
use poise::serenity_prelude as serenity;
use ::serenity::all::GatewayIntents;

use crate::commands::{add_cat::{self}, overview, remove_cat, update};

mod database;
mod commands;
mod utils;
mod types;

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("TOKEN")
        .expect("Missing `TOKEN` env var, see README for more information.");
    let intents = GatewayIntents::GUILD_MEMBERS;
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![ add_cat::add_cat(),
                            remove_cat::remove_cat(), 
                            overview::overview(),
                            update::update()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}