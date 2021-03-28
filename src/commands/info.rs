use crate::{
    server::{Server, ServerStatus},
    settings::Settings,
    state::{Rev, State},
};
use linked_hash_set::LinkedHashSet;
use serenity::{builder::CreateEmbed, prelude::*};
use serenity::{framework::standard::Args, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    utils::MessageBuilder,
};
use std::collections::HashMap;
use std::str::FromStr;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[command]
#[description = "Explains what this bot is about."]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title(format!("Veloren Server Bot v{}", VERSION));
                e.description(
                    MessageBuilder::new()
                        .push("written by ")
                        .mention(&UserId(137581264247980033))
                        .build(),
                );
                e.field(
                    "Purpose of this bot",
                    "Provide easy access to the Veloren testing server.",
                    true,
                );
                e.footer(|f| {
                    f.text(format!(
                        "Copyright © {} Veloren Team",
                        chrono::Utc::now().date().format("%Y")
                    ))
                });
                e
            });
            m
        })
        .await?;
    Ok(())
}

#[derive(Debug)]
pub enum StatusOperation {
    Verbose,
    Normal,
}

impl FromStr for StatusOperation {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "verbose" | "v" => Ok(StatusOperation::Verbose),
            _ => Err("Unknown Operation"),
        }
    }
}

#[command]
#[description = r#"Prints current status of the Veloren Server.
Available subcommands:
`status verbose` - Detailed status."#]
async fn status(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let data = ctx.data.read().await;

    let mut server = data_get!(data, msg, ctx, Server);
    let settings = data_get!(data, msg, ctx, Settings);
    let state = data_get!(data, msg, ctx, State);

    let status = server.status().await;

    let operation = args
        .single::<StatusOperation>()
        .unwrap_or(StatusOperation::Normal);

    match operation {
        StatusOperation::Normal => {
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        create_status_msg(
                            e,
                            &status,
                            server.version(),
                            state.rev(),
                            &settings.gameserver_address,
                            None,
                            None,
                            None,
                        )
                    });
                    m
                })
                .await?;
        }
        StatusOperation::Verbose => {
            msg.channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        create_status_msg(
                            e,
                            &status,
                            server.version(),
                            state.rev(),
                            &settings.gameserver_address,
                            Some(state.envs().clone()),
                            Some(state.args().clone()),
                            Some(state.cargo_args().clone()),
                        )
                    });
                    m
                })
                .await?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_status_msg<'b>(
    e: &'b mut CreateEmbed,
    status: &ServerStatus,
    version: Option<String>,
    rev: &Rev,
    address: &str,
    envs: Option<HashMap<String, String>>,
    args: Option<LinkedHashSet<String>>,
    cargo_args: Option<LinkedHashSet<String>>,
) -> &'b mut CreateEmbed {
    let envs_msg = match envs {
        Some(env) => {
            let mut envs = MessageBuilder::new();
            if env.is_empty() {
                envs.push_italic_line("No envs set.");
            } else {
                for (name, value) in env {
                    envs.push_codeblock_safe(format!("{}={}", name, value), Some("swift"));
                }
            }
            Some(envs.build())
        }
        None => None,
    };

    let args_msg = match args {
        Some(arg) => {
            let mut args = MessageBuilder::new();
            if arg.is_empty() {
                args.push_italic_line("No gameserver arguments set.");
            } else {
                args.push_mono(arg.into_iter().collect::<Vec<String>>().join(" "));
            };
            Some(args.build())
        }
        None => None,
    };

    let cargo_args_msg = match cargo_args {
        Some(cargo_arg) => {
            let mut cargo_args = MessageBuilder::new();
            if cargo_arg.is_empty() {
                cargo_args.push_italic_line("No cargo arguments set.");
            } else {
                cargo_args.push_mono(cargo_arg.into_iter().collect::<Vec<String>>().join(" "));
            };
            Some(cargo_args.build())
        }
        None => None,
    };

    e.title(":bar_chart: Veloren Server Status");
    e.field(
        "Status",
        MessageBuilder::new().push_mono(status).build(),
        true,
    );
    match rev {
        Rev::Branch(branch) => {
            if let Some(version) = version {
                e.field(
                    "Commit",
                    MessageBuilder::new().push_mono(version).build(),
                    true,
                );
            }
            e.field(
                "Branch",
                MessageBuilder::new().push_mono(branch).build(),
                false,
            );
        }
        Rev::Commit(commit) => {
            e.field(
                "Commit",
                MessageBuilder::new().push_mono(commit).build(),
                false,
            );
        }
    }

    if let Some(envs_msg) = envs_msg {
        e.field(":label: Environment variables", envs_msg, false);
    }
    if let Some(args_msg) = args_msg {
        e.field(":video_game: Gameserver arguments", args_msg, false);
    }
    if let Some(cargo_args_msg) = cargo_args_msg {
        e.field(":package: Cargo arguments", cargo_args_msg, false);
    }

    e.field(
        "Address",
        MessageBuilder::new()
            .push_codeblock_safe(&address, None)
            .build(),
        false,
    );
    e.footer(|f| {
        f.text(format!(
            "Copyright © {} Veloren Team",
            chrono::Utc::now().date().format("%Y")
        ))
    });

    e
}
