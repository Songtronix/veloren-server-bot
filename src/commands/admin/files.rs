use crate::discord::Context;
use crate::discord::Error;
use anyhow::Context as AnyhowContext;
use poise::serenity_prelude::Attachment;
use poise::serenity_prelude::AttachmentType;
use poise::serenity_prelude::MessageBuilder;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Debug, poise::ChoiceParameter)]
pub enum File {
    Db,
    Admins,
    Banlist,
    Description,
    Settings,
    Whitelist,
    CliSettings,
}

// TODO: Do not hardcode files & their path.
impl File {
    pub fn path(&self) -> PathBuf {
        match self {
            File::Db => PathBuf::from("veloren/target/debug/userdata/server/saves/db.sqlite"),
            File::Admins => {
                PathBuf::from("veloren/target/debug/userdata/server/server_config/admins.ron")
            }
            File::Banlist => {
                PathBuf::from("veloren/target/debug/userdata/server/server_config/banlist.ron")
            }
            File::Description => {
                PathBuf::from("veloren/target/debug/userdata/server/server_config/description.ron")
            }
            File::Settings => {
                PathBuf::from("veloren/target/debug/userdata/server/server_config/settings.ron")
            }
            File::Whitelist => {
                PathBuf::from("veloren/target/debug/userdata/server/server_config/whitelist.ron")
            }
            File::CliSettings => {
                PathBuf::from("veloren/target/debug/userdata/server-cli/settings.ron")
            }
        }
    }
}

/// Manage Veloren server files.
#[poise::command(
    slash_command,
    check = "crate::checks::is_admin",
    subcommands("upload", "remove", "view")
)]
pub async fn files(_ctx: Context<'_>) -> Result<(), Error> {
    // Discord doesn't allow root commands to be invoked. Only Subcommands.
    Ok(())
}

/// Manage Veloren server files.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn upload(
    ctx: Context<'_>,
    #[description = "which file to upload"] file: File,
    #[description = "which file to upload"] newfile: Attachment,
) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let state = ctx.data().state.lock().await;

    // Note: This will download the file straight to RAM.
    let content = match newfile.download().await {
        Ok(content) => content,
        Err(why) => {
            ctx.say(format!("Error downloading attachment: {}", why))
                .await?;
            return Ok(());
        }
    };

    server.stop().await;

    let mut file = tokio::fs::File::create(file.path())
        .await
        .context("Failed to open file for upload.")?;
    file.write_all(&content)
        .await
        .context("Failed to write file for upload.")?;
    file.sync_all()
        .await
        .context("Failed to sync data for upload.")?;

    server
        .start(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await;

    ctx.say("File uploaded and server restarted.").await?;

    Ok(())
}

/// Manage Veloren server files.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "which file to remove"] file: File,
) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let state = ctx.data().state.lock().await;

    server.stop().await;

    if let Err(e) = tokio::fs::remove_file(file.path()).await {
        ctx.say(format!("Failed to delete file: {}", e)).await?;
        return Ok(());
    }

    server
        .start(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await;

    ctx.say("File removed and server restarted.").await?;

    Ok(())
}

/// Manage Veloren server files.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn view(
    ctx: Context<'_>,
    #[description = "which file to view"] file: File,
) -> Result<(), Error> {
    let path = file.path();

    if file.path().extension().unwrap() == "ron" {
        let content = match tokio::fs::read_to_string(file.path()).await {
            Ok(content) => content,
            Err(e) => {
                ctx.say(format!("Failed to read file: {}", e)).await?;
                return Ok(());
            }
        };

        ctx.say(
            MessageBuilder::new()
                .push_codeblock(content, Some("rust"))
                .build(),
        )
        .await?;
    } else if let Err(e) = ctx
        .send(|m| m.attachment(AttachmentType::Path(&path)).ephemeral(true))
        .await
    {
        ctx.say(format!("Failed to send file: {}", e)).await?;
    }

    Ok(())
}
