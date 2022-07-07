use crate::discord::Context;
use crate::discord::Error;
use crate::{server::ServerStatus, state::Rev};
use linked_hash_set::LinkedHashSet;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::CreateEmbed;
use poise::serenity_prelude::MessageBuilder;
use poise::serenity_prelude::UserId;
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Explains what this bot is about.
#[poise::command(slash_command)]
pub async fn about(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(|m| {
        m.embed(|e| {
            e.title(format!("Veloren Server Bot v{}", VERSION));
            e.description(
                serenity::MessageBuilder::new()
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

/// Prints current status of the Veloren Server.
#[poise::command(slash_command)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let mut server = ctx.data().server.lock().await;
    let settings = ctx.data().settings.lock().await;
    let state = ctx.data().state.lock().await;

    let status = server.status().await;

    ctx.send(|m| {
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

    e
}
