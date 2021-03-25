use serenity::{
    framework::standard::macros::check, framework::standard::Args,
    framework::standard::CommandOptions, framework::standard::Reason, model::channel::Message,
    prelude::*,
};

use crate::settings::Settings;

// A function which acts as a "check", to determine whether to call a command.
//
// This check analyses whether a guild member permissions has
// administrator-permissions.
#[check]
#[name = "Admin"]
// Whether the check shall be tested in the help-system.
#[check_in_help(true)]
// Whether the check shall be displayed in the help-system.
#[display_in_help(false)]
async fn admin_check(
    ctx: &Context,
    msg: &Message,
    _: &mut Args,
    _: &CommandOptions,
) -> Result<(), Reason> {
    let data = ctx.data.read().await;
    if let Some(settings) = data.get::<Settings>() {
        let settings = settings.lock().await;

        if settings.admins().contains(&msg.author.id) || settings.owner == msg.author.id.0 {
            return Ok(());
        } else {
            return Err(Reason::User(
                "You need to be an Admin to execute this command.".to_string(),
            ));
        }
    }

    Err(Reason::User(
        "Failed to aquire admin list. Please contact an admin to report this issue.".to_string(),
    ))
}
