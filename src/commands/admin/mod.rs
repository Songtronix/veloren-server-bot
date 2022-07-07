use poise::serenity_prelude::MessageBuilder;

use crate::discord::Context;
use crate::discord::Error;

pub mod args;
pub mod cargo;
pub mod envs;
pub mod exec;
pub mod files;

/// Switch the revision (Branch/Commit) of the Veloren server. Will restart the server.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn rev(
    ctx: Context<'_>,
    #[description = "Commit or branch to switch to."] rev: String,
) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let settings = ctx.data().settings.lock().await;
    let mut state = ctx.data().state.lock().await;

    let edit_msg = ctx.say("Checking if rev exists...").await?;

    match state.set_rev(&rev, &settings.repository).await? {
        true => {
            edit_msg
                .edit(ctx, |m| {
                    m.content(format!(
                        "Changed to `{}`. Check with `status` for servers' progress.",
                        &rev
                    ))
                })
                .await?;
            server
                .restart(state.rev(), state.args(), state.cargo_args(), state.envs())
                .await;
        }
        false => {
            edit_msg
                .edit(ctx, |m| m.content(format!("`{}` does not exist!", &rev)))
                .await?;
        }
    };

    Ok(())
}

/// Sends you the details to aquire the logs.
#[poise::command(slash_command, ephemeral, check = "crate::checks::is_admin")]
pub async fn logs(ctx: Context<'_>) -> Result<(), Error> {
    let settings = ctx.data().settings.lock().await;

    ctx.send(|m| {
        m.content(
            MessageBuilder::new()
                .push_bold_line("Keep these credentials secure!")
                .push("Username: ")
                .push_mono_line(&settings.web_username)
                .push("Password: ")
                .push_mono_line(&settings.web_password)
                .push("Url: ")
                .push_line(&settings.web_address)
                .build(),
        )
    })
    .await?;

    Ok(())
}

/// Start Veloren Server. Will recompile, change branch/commit, fetch updates as needed.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn start(ctx: Context<'_>) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let state = ctx.data().state.lock().await;

    let resp = match server
        .start(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await
    {
        true => "Started Veloren Server. Check with `status` for its progress.",
        false => "Server is already running.",
    };

    ctx.say(resp).await?;

    Ok(())
}

/// Stop the Veloren server.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;

    let resp = match server.stop().await {
        true => "Stopped the Veloren Server.",
        false => "Server is already stopped.",
    };

    ctx.say(resp).await?;

    Ok(())
}

/// Runs cargo clean and restarts the server.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn prune(ctx: Context<'_>) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let state = ctx.data().state.lock().await;

    match server
        .clean(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await
    {
        true => {
            ctx.say("Cleaned and restarted server.").await?;
        }
        false => {
            ctx.say("Failed to clean. Check the logs for more information.")
                .await?;
        }
    }

    Ok(())
}

/// Restart Veloren Server. Will recompile, change branch/commit, fetch updates as needed.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn restart(ctx: Context<'_>) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let state = ctx.data().state.lock().await;

    server
        .restart(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await;

    ctx.say("Restarted Veloren Server. Check with `status` for its progress.")
        .await?;

    Ok(())
}
