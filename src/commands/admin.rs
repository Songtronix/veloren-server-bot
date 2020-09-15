use crate::{server::Server, settings::Settings};
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::str::FromStr;
use tokio::io::AsyncWriteExt;

#[command]
#[description = "Returns current settings of the Veloren Server"]
async fn settings(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let server = match data.get::<Server>() {
        Some(server) => server.lock().await,
        None => {
            msg.channel_id
                .say(&ctx.http, "Couldn't aquire server information.")
                .await?;
            return Ok(());
        }
    };

    msg.channel_id
        .say(
            &ctx.http,
            MessageBuilder::new()
                .push("Current server settings:")
                .push_codeblock(server.settings().await?, Some("rust"))
                .build(),
        )
        .await?;
    Ok(())
}

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
#[description = r#"Sends you the database of the Veloren server.
Available subcommands:
`db delete`   - Delete the current db.
`db upload`   - Upload and replace the current db.
`db download` - Sends you the db via DM."#]
async fn db(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "Check `help db` to view all available subcommands.",
            )
            .await?;
        return Ok(());
    }

    let operation = args.single::<Operation>()?;

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

    match operation {
        Operation::Download => {
            server.stop().await;

            msg.author
                .dm(&ctx.http, |m| {
                    m.content("Server database:").add_file(server.database())
                })
                .await?;

            if !msg.is_private() {
                msg.channel_id
                    .say(&ctx.http, "Database send via DM.")
                    .await?;
            }

            server.start(settings.branch()).await;
        }
        Operation::Upload => {
            match msg.attachments.iter().find(|s| s.filename == "db.sqlite") {
                Some(attachment) => {
                    let content = match attachment.download().await {
                        Ok(content) => content,
                        Err(why) => {
                            msg.channel_id
                                .say(&ctx, format!("Error downloading attachment: {}", why))
                                .await?;
                            return Ok(());
                        }
                    };
                    server.stop().await;

                    let mut db = tokio::fs::File::create(server.database()).await?;
                    db.write_all(&content).await?;
                    db.sync_all().await?;

                    server.start(settings.branch()).await;

                    msg.channel_id.say(&ctx, "Database uploaded.").await?;
                }
                None => {
                    msg.channel_id
                        .say(
                            &ctx.http,
                            "Please attach the db to the message containing the upload command.",
                        )
                        .await?;
                }
            };
        }
        Operation::Delete => {
            server.stop().await;
            match tokio::fs::remove_file(server.database()).await {
                Ok(_) => {
                    msg.channel_id.say(&ctx.http, "Deleted database.").await?;
                }
                Err(e) => {
                    msg.channel_id
                        .say(&ctx.http, format!("Error: {}", e))
                        .await?;
                }
            }
            server.start(settings.branch()).await;
        }
    };

    Ok(())
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
