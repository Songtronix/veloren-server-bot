use crate::discord::Context;
use crate::discord::Error;
use poise::serenity_prelude::MessageBuilder;

/// Manage environment variables passed to the gameserver.
#[poise::command(
    slash_command,
    check = "crate::checks::is_admin",
    subcommands("set", "remove", "list", "reset")
)]
pub async fn envs(_ctx: Context<'_>) -> Result<(), Error> {
    // Discord doesn't allow root commands to be invoked. Only Subcommands.
    Ok(())
}

#[derive(Debug, poise::Modal)]
struct EnvVar {
    name: String,
    value: String,
}

/// Set an evironment variable.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Environment Variable name"] name: String,
    #[description = "Environment Variable value"] value: String,
) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;
    state.add_env(&name, &value).await?;

    ctx.say(format!(
        "Set `{}`=`{}` as environment variable.",
        name, value
    ))
    .await?;

    Ok(())
}

/// Remove an Environment Variable
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Environment Variable value to remove"] name: String,
) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.remove_env(&name).await?;
    ctx.say(format!(
        "Removed `{}` from the environment variables.",
        name
    ))
    .await?;

    Ok(())
}

/// List all Environment Variables
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let state = ctx.data().state.lock().await;

    let mut response = MessageBuilder::new();
    response.push_bold_line("Environment variables:");
    for (env, value) in state.envs() {
        response.push_mono_line_safe(format!("{} : {}", env, value));
    }
    if state.envs().is_empty() {
        response.push_italic_line("No environment variables set.");
    }
    ctx.say(response.build()).await?;
    Ok(())
}

/// Reset all Environment Variables to default.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn reset(ctx: Context<'_>) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.reset_envs().await?;
    ctx.say("Reset all environment variables to default.")
        .await?;
    Ok(())
}
