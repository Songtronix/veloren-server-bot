use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::standard::macros::hook,
    framework::standard::DispatchError,
    framework::standard::Reason,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::channel::Message,
    model::id::UserId,
    model::prelude::OnlineStatus,
    model::{event::ResumedEvent, gateway::Ready, prelude::Activity},
    prelude::*,
};
use std::{collections::HashSet, sync::Arc};

use crate::{checks::*, commands::*, server::Server, settings::Settings, state::State, Result};

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler(String);

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        log::info!("Connected as {}", ready.user.name);
        ctx.set_presence(None, OnlineStatus::Online).await;
        ctx.set_activity(Activity::playing(&self.0)).await;
    }

    async fn resume(&self, _ctx: Context, _: ResumedEvent) {
        log::info!("Connection to discord resumed.");
    }
}

#[group]
#[commands(about, status)]
#[description = "General information about the bot/server."]
struct Info;

#[group]
#[commands(start, stop, restart, branch, logs, files)]
#[checks(Admin)]
#[description = "All veloren server related commands."]
struct Admin;

#[group]
#[commands(admin, quit)]
#[owners_only]
#[description = "Commands which only the Bot Owner can execute."]
struct Owner;

pub async fn run(settings: Settings, server: Server) -> Result<()> {
    let http = Http::new_with_token(&settings.token);

    // Aquire bot id
    let bot_id = match http.get_current_application_info().await {
        Ok(info) => info.id,
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let mut owners = HashSet::new();
    owners.insert(UserId(settings.owner));

    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners)
                .prefix(&settings.prefix)
                .on_mention(Some(bot_id))
                .case_insensitivity(true)
                .allow_dm(true)
                .no_dm_prefix(true)
        })
        .group(&INFO_GROUP)
        .group(&ADMIN_GROUP)
        .group(&OWNER_GROUP)
        .before(before_hook)
        .on_dispatch_error(dispatch_error_hook)
        .help(&HELP);

    let mut client = Client::builder(&settings.token)
        .event_handler(Handler(settings.gameserver_address.clone()))
        .framework(framework)
        .await?;

    let state = State::new().unwrap_or_default();

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
        data.insert::<Settings>(Arc::new(Mutex::new(settings)));
        data.insert::<State>(Arc::new(Mutex::new(state)));
        data.insert::<Server>(Arc::new(Mutex::new(server)));
    }

    Ok(client.start().await?)
}

#[hook]
async fn before_hook(ctx: &Context, msg: &Message, _cmd_name: &str) -> bool {
    log::info!(
        "Got command '{}' by user '{}'",
        msg.content_safe(&ctx.cache).await,
        msg.author.tag()
    );

    true
}

#[hook]
async fn dispatch_error_hook(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        DispatchError::NotEnoughArguments { min, given } => {
            let s = format!("Need {} arguments, but only got {}.", min, given);
            let _ = msg.channel_id.say(&ctx, &s).await;
        }
        DispatchError::TooManyArguments { max, given } => {
            let s = format!("Max arguments allowed is {}, but got {}.", max, given);
            let _ = msg.channel_id.say(&ctx, &s).await;
        }
        DispatchError::CheckFailed(_failed_check, Reason::User(reason)) => {
            let _ = msg.channel_id.say(&ctx.http, reason).await;
        }
        e => log::warn!("Unhandled dispatch error. {:?}", e),
    }
}
