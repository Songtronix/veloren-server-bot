use crate::state::State;
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::{collections::HashSet, str::FromStr};

#[derive(Debug)]
pub enum CargoOperation {
    Add,
    Remove,
    Clear,
    List,
}

impl FromStr for CargoOperation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "add" => Ok(CargoOperation::Add),
            "remove" | "rm" => Ok(CargoOperation::Remove),
            "list" | "ls" => Ok(CargoOperation::List),
            "clear" | "reset" => Ok(CargoOperation::Clear),
            _ => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = r#"Manage arguments passed to cargo.
Available subcommands:
`cargo <arguments> - Add multiple cargo arguments.
`cargo add <VALUE>` - Add an cargo argument.
`cargo remove/rm <NAME>` - Remove an cargo argument.
`cargo list/ls` - List all cargo arguments.
`cargo clear - Removes all cargo arguments.`"#]
async fn cargo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;
    let mut state = data_get!(data, msg, ctx, State);

    let operation = match args.single::<CargoOperation>() {
        Ok(op) => op,
        Err(_) => {
            if args.remains().is_none() {
                CargoOperation::List
            } else {
                let mut all_args = HashSet::new();
                for arg in args.iter::<String>().flatten() {
                    all_args.insert(arg);
                }
                state.add_cargo_args(all_args).await?;
                msg.channel_id
                    .say(&ctx.http, "Added all as argument.")
                    .await?;
                return Ok(());
            }
        }
    };

    match operation {
        CargoOperation::List => {
            let mut response = MessageBuilder::new();
            response.push_bold_line("Cargo Arguments:");
            for arg in state.cargo_args() {
                response.push_mono_line_safe(arg);
            }
            if state.cargo_args().is_empty() {
                response.push_italic_line("No cargo arguments set.");
            }
            msg.channel_id.say(&ctx.http, response.build()).await?;
        }
        CargoOperation::Add => {
            let arg = args.single::<String>()?;

            state.add_cargo_arg(&arg).await?;
            msg.channel_id
                .say(&ctx.http, format!("Added `{}` as cargo argument.", arg))
                .await?;
        }
        CargoOperation::Remove => {
            let arg = args.single::<String>()?;

            state.remove_cargo_arg(&arg).await?;
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Removed `{}` from the cargo arguments.", arg),
                )
                .await?;
        }
        CargoOperation::Clear => {
            state.clear_cargo_args().await?;
            msg.channel_id
                .say(&ctx.http, "Reset all cargo arguments to default.")
                .await?;
        }
    };

    Ok(())
}
