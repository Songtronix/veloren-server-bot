use crate::state::State;
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::str::FromStr;

#[derive(Debug)]
pub enum EnvOperation {
    Set,
    Remove,
    List,
    Reset,
}

impl FromStr for EnvOperation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "set" => Ok(EnvOperation::Set),
            "remove" | "rm" => Ok(EnvOperation::Remove),
            "list" | "ls" => Ok(EnvOperation::List),
            "clear" | "reset" => Ok(EnvOperation::Reset),
            _ => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = r#"Manage environment variables passed to the gameserver.
Available subcommands:
`envs set <NAME> <VALUE>` - Add an environment variable.
`envs remove/rm <NAME>` - Remove an environment variable.
`envs list/ls` - List all environment variables.
`envs reset/clear - Resets all environment variables to default.`"#]
async fn envs(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let operation = args.single::<EnvOperation>().unwrap_or(EnvOperation::List);

    let data = ctx.data.read().await;
    let mut state = data_get!(data, msg, ctx, State);

    match operation {
        EnvOperation::List => {
            let mut response = MessageBuilder::new();
            response.push_bold_line("Environment variables:");
            for (env, value) in state.envs() {
                response.push_mono_line_safe(format!("{} : {}", env, value));
            }
            if state.envs().is_empty() {
                response.push_italic_line("No environment variables set.");
            }
            msg.channel_id.say(&ctx.http, response.build()).await?;
        }
        EnvOperation::Set => {
            let name = args.single::<String>()?;
            let value = args.single::<String>()?;

            state.add_env(&name, &value).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Set `{}`=`{}` as environment variable.", name, value),
                )
                .await?;
        }
        EnvOperation::Remove => {
            let name = args.single::<String>()?;

            state.remove_env(&name).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Removed `{}` from the environment variables.", name),
                )
                .await?;
        }
        EnvOperation::Reset => {
            state.reset_envs().await?;
            msg.channel_id
                .say(&ctx.http, "Reset all environment variables to default.")
                .await?;
        }
    };

    Ok(())
}
