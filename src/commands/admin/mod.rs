use crate::{server::Server, settings::Settings, state::State};
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};

mod args;
mod cargo;
mod envs;

pub use args::*;
pub use cargo::*;
pub use envs::*;

#[command]
#[aliases("branch", "commit")]
#[description = "Switch the revision (Branch/Commit) of the Veloren server. Will restart the server."]
async fn rev(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let rev = args.single::<String>()?;

    let data = ctx.data.read().await;
    let mut server = data_get!(data, msg, ctx, Server);
    let mut state = data_get!(data, msg, ctx, State);

    let mut edit_msg = msg
        .channel_id
        .say(&ctx.http, "Checking if rev exists...")
        .await?;

    match state.set_rev(&rev).await? {
        true => {
            edit_msg
                .edit(&ctx.http, |m| {
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
                .edit(&ctx.http, |m| {
                    m.content(format!("`{}` does not exist!", &rev))
                })
                .await?;
        }
    };

    Ok(())
}

#[command]
#[description = "Sends you the details to aquire the logs."]
async fn logs(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let settings = data_get!(data, msg, ctx, Settings);

    msg.author
        .dm(&ctx, |m| {
            m.content(
                MessageBuilder::new()
                    .push_bold_line("Keep these credentials secure!")
                    .push_bold("Username: ")
                    .push_line(&settings.web_username)
                    .push_bold("Password: ")
                    .push_mono(&settings.web_password)
                    .push_bold("Url: ")
                    .push_mono(&settings.web_address)
                    .build(),
            )
        })
        .await?;

    Ok(())
}

#[command]
#[description = "Start Veloren Server. Will recompile, change branch/commit, fetch updates as needed."]
async fn start(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let mut server = data_get!(data, msg, ctx, Server);
    let state = data_get!(data, msg, ctx, State);

    server
        .start(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await;

    msg.channel_id
        .say(
            &ctx.http,
            "Started Veloren Server. Check with `status` for its progress.",
        )
        .await?;

    Ok(())
}

#[command]
#[description = "Stop the Veloren server."]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let mut server = data_get!(data, msg, ctx, Server);

    server.stop().await;

    msg.channel_id
        .say(&ctx.http, "Stopped the Veloren Server.")
        .await?;

    Ok(())
}

#[command]
#[description = "Runs cargo clean and restarts the server."]
async fn prune(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let mut server = data_get!(data, msg, ctx, Server);
    let state = data_get!(data, msg, ctx, State);

    match server
        .clean(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await
    {
        true => {
            msg.channel_id
                .say(&ctx.http, "Cleaned and restarted server.")
                .await?;
        }
        false => {
            msg.channel_id
                .say(
                    &ctx.http,
                    "Failed to clean. Check the logs for more information.",
                )
                .await?;
        }
    }

    Ok(())
}

#[command]
#[description = "Restart Veloren Server. Will recompile, change branch/commit, fetch updates as needed."]
async fn restart(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;

    let mut server = data_get!(data, msg, ctx, Server);
    let state = data_get!(data, msg, ctx, State);

    server
        .restart(state.rev(), state.args(), state.cargo_args(), state.envs())
        .await;

    msg.channel_id
        .say(
            &ctx.http,
            "Restarted Veloren Server. Check with `status` for its progress.",
        )
        .await?;

    Ok(())
}
