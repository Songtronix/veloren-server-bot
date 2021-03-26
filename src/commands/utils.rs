use anyhow::Result;
use serenity::{client::Context, model::channel::Message, model::id::UserId, model::prelude::User};

/// Tries to find the User based on the three possible identifiers:
///
/// Mention:
/// @someone
///
/// Tag:
/// Username#4523
///
/// Id:
/// 43564354543
pub async fn get_member(ctx: &Context, msg: &Message, identifier: &str) -> Result<Option<User>> {
    // Id
    if let Ok(id) = identifier.parse::<u64>() {
        return Ok(Some(UserId(id).to_user(&ctx.http).await?));
    }
    // Mention
    if let Some(user) = msg.mentions.first() {
        return Ok(Some(user.clone()));
    }
    // Tag
    if let Some(guild) = msg.guild(&ctx.cache).await {
        if let Some(member) = guild.member_named(identifier) {
            return Ok(Some(member.user.clone()));
        }
    }

    Ok(None)
}

#[macro_use]
mod macros {
    macro_rules! data_get {
        ($data:ident, $msg:ident, $ctx:ident,$x:ty) => {
            match $data.get::<$x>() {
                Some(x) => x.lock().await,
                None => {
                    $msg.channel_id
                        .say(&$ctx, "There was a problem getting the $x :/")
                        .await?;
                    return Ok(());
                }
            }
        };
    }
}
