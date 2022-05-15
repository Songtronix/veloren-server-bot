use crate::{commands::*, server::Server, settings::Settings, state::State, Result};
use poise::serenity_prelude::{self as serenity, Activity, OnlineStatus};
use tokio::sync::Mutex;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    pub settings: Mutex<Settings>,
    pub state: Mutex<State>,
    pub server: Mutex<Server>,
}

async fn event_listener(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    user_data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            log::info!("Connected as {}", data_about_bot.user.name);
            ctx.set_presence(None, OnlineStatus::Online).await;
            ctx.set_activity(Activity::playing(
                user_data.settings.lock().await.gameserver_address.clone(),
            ))
            .await;
        }
        poise::Event::Resume { event: _ } => {
            log::info!("Connection to discord resumed.");
        }
        _ => {}
    }

    Ok(())
}

pub async fn run(settings: Settings, server: Server) -> Result<()> {
    let options = poise::FrameworkOptions {
        commands: vec![
            info::about(),
            info::status(),
            help::help(),
            owner::quit(),
            owner::admin(),
            owner::register(),
            admin::rev(),
            admin::logs(),
            admin::start(),
            admin::stop(),
            admin::prune(),
            admin::restart(),
            admin::exec::exec(),
            admin::args::args(),
            admin::cargo::cargo(),
            admin::envs::envs(),
            admin::files::files(),
        ],
        listener: |ctx, event, framework, user_data| {
            Box::pin(event_listener(ctx, event, framework, user_data))
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| Box::pin(pre_command(ctx)),
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(String::from("~")),
            mention_as_prefix: false,
            edit_tracker: Some(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(3600 * 3),
            )),
            ..Default::default()
        },

        ..Default::default()
    };

    let state = match State::new() {
        Ok(state) => state,
        Err(_) => {
            let state = State::default();
            state.save().await?;
            state
        }
    };

    poise::Framework::build()
        .token(&settings.token)
        .user_data_setup(move |_ctx, _ready, _framework| Box::pin(async move {
            Ok(
                Data { settings: Mutex::new(settings),
                    state: Mutex::new(state),
                    server: Mutex::new(server)
                }
            )
        }))
        .options(options)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT, /* TODO: Remove MESSAGE_CONTENT INTENT  */
        )
        .run()
        .await
        .unwrap();

    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            log::error!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                log::error!("Error while handling error: {}", e)
            }
        }
    }
}

async fn pre_command(ctx: Context<'_>) {
    log::info!(
        "Got command '{}' by user '{}'",
        ctx.command().name,
        ctx.author().tag()
    );
}
