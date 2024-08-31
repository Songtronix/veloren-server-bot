use crate::discord::Context;
use crate::discord::Error;
use anyhow::Result;
use poise::serenity_prelude::MessageBuilder;
use poise::serenity_prelude::OnlineStatus;
use poise::serenity_prelude::User;

/// Shutdown the bot.
#[poise::command(slash_command, check = "crate::checks::is_owner")]
pub async fn quit(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Shutting down!").await?;
    ctx.serenity_context()
        .set_presence(None, OnlineStatus::Offline);
    ctx.framework().shard_manager().shutdown_all().await;

    Ok(())
}

/// Manage admins which are able to modify the server.
#[poise::command(
    slash_command,
    check = "crate::checks::is_admin",
    subcommands("add", "remove", "list")
)]
pub async fn admin(_ctx: Context<'_>) -> Result<(), Error> {
    // Discord doesn't allow root commands to be invoked. Only Subcommands.
    Ok(())
}

/// Manage admins which are able to modify the server.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn add(
    ctx: Context<'_>,
    #[description = "User to add to the admin list"] user: User,
) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.add_admin(user.id.get()).await?;
    ctx.say(format!("Added '{}' to the admins list.", user.tag()))
        .await?;

    Ok(())
}

/// Manage admins which are able to modify the server.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "User to remove from the admin list"] user: User,
) -> Result<(), Error> {
    let mut state = ctx.data().state.lock().await;

    state.remove_admin(user.id.get()).await?;
    ctx.say(format!("Removed '{}' from the admins list.", user.tag()))
        .await?;

    Ok(())
}

/// Manage admins which are able to modify the server.
#[poise::command(slash_command, check = "crate::checks::is_admin")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let state = ctx.data().state.lock().await;

    let mut response = MessageBuilder::new();
    response.push_bold_line("Admins:");
    for admin in state.admins() {
        let admin = admin.to_user(&ctx.serenity_context().http).await?;
        response.push_line_safe(format!("{} ({})", admin.tag(), admin.id));
    }
    if state.admins().is_empty() {
        response.push_italic_line("No Admins found.");
    }
    ctx.say(response.build()).await?;

    Ok(())
}
