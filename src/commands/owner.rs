use super::utils;
use crate::{discord::ShardManagerContainer, state::State};
use anyhow::Result;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    utils::MessageBuilder,
};
use std::str::FromStr;

#[command]
#[description = "Shutdown the bot."]
async fn quit(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.channel_id.say(&ctx, "Shutting down!").await?;
        ctx.set_presence(None, OnlineStatus::Offline).await;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(&ctx, "There was a problem getting the shard manager")
            .await?;
        return Ok(());
    }

    Ok(())
}

#[derive(Debug)]
pub enum Operation {
    Add,
    Remove,
    List,
}

impl FromStr for Operation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "add" => Ok(Operation::Add),
            "remove" | "rm" => Ok(Operation::Remove),
            "list" | "ls" => Ok(Operation::List),
            _ => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = r#"Manage admins which are able to modify the server.
Available subcommands:
`admin add` - Add an admin.
`admin remove/rm` - Remove an admin.
`admin list/ls` - List all admins."#]
async fn admin(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "Check `help admin` to view all available subcommands.",
            )
            .await?;
        return Ok(());
    }
    let operation = args.single::<Operation>()?;

    let data = ctx.data.read().await;
    let mut state = data_get!(data, msg, ctx, State);

    match operation {
        Operation::List => {
            let mut response = MessageBuilder::new();
            response.push_bold_line("Admins:");
            for admin in state.admins() {
                let admin = admin.to_user(&ctx.http).await?;
                response.push_line_safe(format!("{} ({})", admin.tag(), admin.id));
            }
            if state.admins().is_empty() {
                response.push_italic_line("No Admins found.");
            }
            msg.channel_id.say(&ctx.http, response.build()).await?;
        }
        Operation::Add => {
            let identifier = args.single::<String>()?;
            let whom = match utils::get_member(&ctx, &msg, &identifier).await? {
                Some(user) => user,
                None => {
                    msg.channel_id
                        .say(&ctx, format!("Couldn't find '{}'", identifier))
                        .await?;
                    return Ok(());
                }
            };

            state.add_admin(whom.id.0).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Added '{}' to the admins list.", whom.tag()),
                )
                .await?;
        }
        Operation::Remove => {
            let identifier = args.single::<String>()?;
            let whom = match utils::get_member(&ctx, &msg, &identifier).await? {
                Some(user) => user,
                None => {
                    msg.channel_id
                        .say(&ctx, format!("Couldn't find '{}'", identifier))
                        .await?;
                    return Ok(());
                }
            };

            state.remove_admin(whom.id.0).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Removed '{}' from the admins list.", whom.tag()),
                )
                .await?;
        }
    };

    Ok(())
}
