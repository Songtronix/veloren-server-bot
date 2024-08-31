use crate::discord::Context;
use crate::discord::Error;

/// Checks whether the user is in the admin list.
pub async fn is_admin(ctx: Context<'_>) -> Result<bool, Error> {
    let state = ctx.data().state.lock().await;
    let settings = ctx.data().settings.lock().await;

    if state.admins().contains(&ctx.author().id) || settings.owner == ctx.author().id.get() {
        Ok(true)
    } else {
        ctx.say("You need to be an Admin to execute this command.")
            .await?;
        Ok(false)
    }
}

/// Checks whether the user is the bot owner.
pub async fn is_owner(ctx: Context<'_>) -> Result<bool, Error> {
    let settings = ctx.data().settings.lock().await;

    if settings.owner == ctx.author().id.get() {
        Ok(true)
    } else {
        ctx.say("You need to be the bot owner to execute this command.")
            .await?;
        Ok(false)
    }
}
