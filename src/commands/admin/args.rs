use crate::state::State;
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::{collections::HashSet, str::FromStr};

#[derive(Debug)]
pub enum ArgOperation {
    Add,
    Remove,
    List,
    Reset,
}

impl FromStr for ArgOperation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "add" => Ok(ArgOperation::Add),
            "remove" | "rm" => Ok(ArgOperation::Remove),
            "list" | "ls" => Ok(ArgOperation::List),
            "clear" | "reset" => Ok(ArgOperation::Reset),
            _ => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = r#"Manage arguments passed to the gameserver.
Available subcommands:
`args <arguments> - Add multiple gameserver arguments.
`args add <VALUE>` - Add an gameserver argument.
`args remove/rm <NAME>` - Remove an gameserver argument.
`args list/ls` - List all gameserver arguments.
`args reset/clear - Resets all gameserver arguments to default.`"#]
async fn args(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let mut state = data_get!(data, msg, ctx, State);

    let operation = match args.single::<ArgOperation>() {
        Ok(op) => op,
        Err(_) => {
            if args.remains().is_none() {
                ArgOperation::List
            } else {
                let mut all_args = HashSet::new();
                for arg in args.iter::<String>().flatten() {
                    all_args.insert(arg);
                }
                state.add_args(all_args).await?;
                msg.channel_id
                    .say(&ctx.http, "Added all as gameserver argument.")
                    .await?;
                return Ok(());
            }
        }
    };

    match operation {
        ArgOperation::List => {
            let mut response = MessageBuilder::new();
            response.push_bold_line("Gameserver Arguments:");
            for arg in state.args() {
                response.push_mono_line_safe(arg);
            }

            if state.args().is_empty() {
                response.push_italic_line("No gameserver arguments set.");
            }
            msg.channel_id.say(&ctx.http, response.build()).await?;
        }
        ArgOperation::Add => {
            let arg = args.single::<String>()?;

            state.add_arg(&arg).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Added `{}` as gameserver argument.", arg),
                )
                .await?;
        }
        ArgOperation::Remove => {
            let arg = args.single::<String>()?;

            state.remove_arg(&arg).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Removed `{}` from the gameserver arguments.", arg),
                )
                .await?;
        }
        ArgOperation::Reset => {
            state.reset_args().await?;
            msg.channel_id
                .say(&ctx.http, "Reset all gameserver arguments to default.")
                .await?;
        }
    };

    Ok(())
}
