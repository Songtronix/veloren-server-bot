use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::collections::HashMap;
use std::{ffi::OsString, path::PathBuf, str::FromStr};
use tokio::io::AsyncWriteExt;

use crate::{server::Server, settings::Settings};

lazy_static::lazy_static! {
    /// All files allowed to be viewed, updated, deleted.
    static ref FILES: HashMap<&'static str, PathBuf> = vec![
        ("db" ,PathBuf::from("veloren/userdata/server/saves/db.sqlite")),
        ("admins", PathBuf::from("veloren/userdata/server/server_config/admins.ron")),
        ("banlist", PathBuf::from("veloren/userdata/server/server_config/banlist.ron")),
        ("description", PathBuf::from("veloren/userdata/server/server_config/description.ron")),
        ("settings", PathBuf::from("veloren/userdata/server/server_config/settings.ron")),
        ("whitelist", PathBuf::from("veloren/userdata/server/server_config/whitelist.ron")),
        ("cli_settings", PathBuf::from("veloren/userdata/server-cli/settings.ron")),
    ].into_iter().collect();
}

#[derive(Debug)]
pub enum Operation {
    Upload,
    Remove,
    View,
    List,
}

impl FromStr for Operation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "upload" | "up" => Ok(Operation::Upload),
            "remove" | "rm" => Ok(Operation::Remove),
            "view" | "v" => Ok(Operation::View),
            "list" | "ls" => Ok(Operation::List),
            _ => Err("Unknown Operation"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct File<'a>(&'a PathBuf);

impl<'a> FromStr for File<'a> {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match FILES.get(s.to_lowercase().as_str()) {
            Some(file) => Ok(File(file)),
            None => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = r#"Manage Veloren server files.
Available subcommands:
`files list/ls` - List files which can be uploaded/removed/viewed.
`files upload/up <file>` - Uploades the file and restarts the server.
`files remove/rm <file>` - Removes the file and restarts the server.
`files view/v <file>` - View the file (will dm you the file incase it's not text)"#]
async fn files(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.is_empty() {
        msg.channel_id
            .say(
                &ctx.http,
                "Check `help files` to view all available subcommands.",
            )
            .await?;
        return Ok(());
    }
    let operation = args.single::<Operation>()?;

    if matches!(operation, Operation::List) {
        let mut response = MessageBuilder::new();
        response.push_bold_line("Files:");
        for file in FILES.keys() {
            response.push_line_safe(file);
        }
        msg.channel_id.say(&ctx.http, response.build()).await?;
        return Ok(());
    }

    let file = match args.single::<File>() {
        Ok(file) => file,
        Err(_) => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Unknown file! Check `files list` for available files.",
                )
                .await?;
            return Ok(());
        }
    };

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
        Operation::Upload => {
            match msg
                .attachments
                .iter()
                .find(|s| OsString::from(&s.filename) == file.0.file_name().unwrap()) // Only files in FILES
            {
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

                    let mut file = tokio::fs::File::create(file.0).await?;
                    file.write_all(&content).await?;
                    file.sync_all().await?;

                    server.start(settings.branch()).await;

                    msg.channel_id.say(&ctx, "File uploaded and server restarted.").await?;
                }
                None => {
                    msg.channel_id
                        .say(
                            &ctx.http,
                            "Please attach the file to the message containing the upload command.",
                        )
                        .await?;
                }
            };
        }
        Operation::Remove => {
            server.stop().await;
            tokio::fs::remove_file(file.0).await?;
            server.start(settings.branch()).await;

            msg.channel_id
                .say(&ctx, "File removed and server restarted.")
                .await?;
        }
        Operation::View => {
            if file.0.extension().unwrap() == "ron" {
                let content = match tokio::fs::read_to_string(file.0).await {
                    Ok(content) => content,
                    Err(e) => {
                        msg.channel_id
                            .say(&ctx, format!("Failed to read file: {}", e))
                            .await?;
                        return Ok(());
                    }
                };

                msg.channel_id
                    .say(
                        &ctx.http,
                        MessageBuilder::new()
                            .push_codeblock(content, Some("rust"))
                            .build(),
                    )
                    .await?;
            } else {
                msg.author
                    .dm(&ctx.http, |m| m.content("File:").add_file(file.0))
                    .await?;

                if !msg.is_private() {
                    msg.channel_id.say(&ctx.http, "File send via DM.").await?;
                }
            }
        }
        Operation::List => unreachable!(),
    };

    Ok(())
}
