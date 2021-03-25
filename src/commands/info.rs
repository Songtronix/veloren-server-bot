use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};

use crate::{server::Server, settings::Settings};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[command]
#[description = "Explains what this bot is about."]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title(format!("Veloren Server Bot v{}", VERSION));
                e.description(
                    MessageBuilder::new()
                        .push("written by ")
                        .mention(&UserId(137581264247980033))
                        .build(),
                );
                e.field(
                    "Purpose of this bot",
                    "Provide easy access to the Veloren testing server.",
                    true,
                );
                e.footer(|f| {
                    f.text(format!(
                        "Copyright © {} Veloren Team",
                        chrono::Utc::now().date().format("%Y")
                    ))
                });
                e
            });
            m
        })
        .await?;
    Ok(())
}

#[command]
#[description = "Prints current status of the Veloren Server"]
async fn status(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let mut server = match data.get::<Server>() {
        Some(server) => server.lock().await,
        None => {
            msg.channel_id
                .say(&ctx.http, "Couldn't aquire server information.")
                .await?;
            return Ok(());
        }
    };
    let settings = match data.get::<Settings>() {
        Some(settings) => settings.lock().await,
        None => {
            msg.channel_id
                .say(&ctx, "There was a problem getting the settings :/")
                .await?;
            return Ok(());
        }
    };

    let status = server.status().await;

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Veloren Server Status");
                e.field(
                    "Status",
                    MessageBuilder::new().push_mono(status).build(),
                    true,
                );
                if let Some(version) = server.version() {
                    e.field(
                        "Commit",
                        MessageBuilder::new().push_mono(version).build(),
                        true,
                    );
                }
                e.field(
                    "Branch",
                    MessageBuilder::new().push_mono(settings.branch()).build(),
                    false,
                );
                e.field(
                    "Address",
                    MessageBuilder::new()
                        .push_codeblock_safe(&settings.address, None)
                        .build(),
                    false,
                );
                e.footer(|f| {
                    f.text(format!(
                        "Copyright © {} Veloren Team",
                        chrono::Utc::now().date().format("%Y")
                    ))
                });
                e
            });
            m
        })
        .await?;
    Ok(())
}
