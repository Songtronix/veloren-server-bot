use crate::discord::Context;
use crate::discord::Error;
use poise::serenity_prelude::MessageBuilder;

/// Manage arguments passed to the gameserver.
#[poise::command(
    slash_command,
    check = "crate::checks::is_admin",
    subcommands("add", "remove", "list", "reset")
)]
pub async fn args(_ctx: Context<'_>) -> Result<(), Error> {
    // Discord doesn't allow root commands to be invoked. Only Subcommands.
    Ok(())
}

/// Add argument passed to the gameserver.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn add(
    ctx: Context<'_>,
    #[description = "argument to add"] argument: String,
) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.add_arg(&argument).await?;
    ctx.say(format!("Added `{}` as gameserver argument.", argument))
        .await?;

    Ok(())
}

/// Remove argument passed to the gameserver.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "argument to remove"] argument: String,
) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.remove_arg(&argument).await?;
    ctx.say(format!(
        "Removed `{}` from the gameserver arguments.",
        argument
    ))
    .await?;

    Ok(())
}

/// List arguments passed to the gameserver.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let state = ctx.data().state.lock().await;

    let mut response = MessageBuilder::new();
    response.push_bold_line("Gameserver Arguments:");
    for arg in state.args() {
        response.push_mono_line_safe(arg);
    }

    if state.args().is_empty() {
        response.push_italic_line("No gameserver arguments set.");
    }

    ctx.say(response.build()).await?;

    Ok(())
}

/// Reset arguments passed to the gameserver to default.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn reset(ctx: Context<'_>) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.reset_args().await?;
    ctx.say("Reset all gameserver arguments to default.")
        .await?;

    Ok(())
}
