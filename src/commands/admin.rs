use crate::{server::Server, settings::Settings};
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::str::FromStr;

#[derive(Debug)]
pub enum Operation {
    Upload,
    Delete,
    Download,
}

impl FromStr for Operation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "upload" => Ok(Operation::Upload),
            "delete" => Ok(Operation::Delete),
            "download" => Ok(Operation::Download),
            _ => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = "Switch the branch of the Veloren server. Will restart the server."]
async fn branch(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let branch = args.single::<String>()?;

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
    let mut settings = match data.get::<Settings>() {
        Some(settings) => settings.lock().await,
        None => {
            msg.channel_id
                .say(&ctx, "There was a problem getting the settings :/")
                .await?;
            return Ok(());
        }
    };

    let mut edit_msg = msg
        .channel_id
        .say(&ctx.http, "Checking if branch exists...")
        .await?;

    match settings.set_branch(&branch).await? {
        true => {
            edit_msg
                .edit(&ctx.http, |m| {
                    m.content(format!(
                        "Changed to branch '{}'. Check with `status` for servers' progress.",
                        &branch
                    ))
                })
                .await?;
            server.restart(settings.branch()).await;
        }
        false => {
            edit_msg
                .edit(&ctx.http, |m| {
                    m.content(format!("Branch '{}' does not exist!", &branch))
                })
                .await?;
        }
    };

    Ok(())
}

#[command]
#[description = "Sends you the details to aquire the logs."]
async fn logs(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let settings = match data.get::<Settings>() {
        Some(settings) => settings.lock().await,
        None => {
            msg.channel_id
                .say(&ctx, "There was a problem getting the settings :/")
                .await?;
            return Ok(());
        }
    };

    msg.author
        .dm(&ctx, |m| {
            m.content(
                MessageBuilder::new()
                    .push_bold_line("Keep these credentials secure!")
                    .push_bold("Username: ")
                    .push_line("Bot")
                    .push_bold("Password: ")
                    .push_line(&settings.web_password)
                    .push_bold("Url: ")
                    .push_line(&settings.web_address)
                    .build(),
            )
        })
        .await?;

    Ok(())
}

#[command]
#[description = "Start Veloren Server. Will recompile, change branch, fetch updates as needed."]
async fn start(ctx: &Context, msg: &Message) -> CommandResult {
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

    server.start(settings.branch()).await;

    msg.channel_id
        .say(
            &ctx.http,
            "Started Veloren Server. Check with `status` for its progress.",
        )
        .await?;

    Ok(())
}

#[command]
#[description = "Stop the Veloren server."]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
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

    server.stop().await;

    msg.channel_id
        .say(&ctx.http, "Stopped the Veloren Server.")
        .await?;

    Ok(())
}

#[command]
#[description = "Restart Veloren Server. Will recompile, change branch, fetch updates as needed."]
async fn restart(ctx: &Context, msg: &Message) -> CommandResult {
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

    server.restart(settings.branch()).await;

    msg.channel_id
        .say(
            &ctx.http,
            "Restarted Veloren Server. Check with `status` for it's progress.",
        )
        .await?;

    Ok(())
}
