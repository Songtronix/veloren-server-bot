use crate::discord::Context;
use crate::discord::Error;

/// Execute a Veloren cli command.
#[poise::command(slash_command, hide_in_help)]
pub async fn exec(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("This command is still WIP, sorry!").await?;

    Ok(())
}
