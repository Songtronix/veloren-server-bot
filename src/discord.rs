use crate::{commands::*, server::Server, settings::Settings, state::State, Result};
use poise::serenity_prelude::{self as serenity, ActivityData, CacheHttp, OnlineStatus};
use tokio::sync::Mutex;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    pub settings: Mutex<Settings>,
    pub state: Mutex<State>,
    pub server: Mutex<Server>,
}

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Ready { data_about_bot } => {
            poise::builtins::register_globally(ctx.http(), &framework.options().commands).await?;

            log::info!("Connected as {}", data_about_bot.user.name);
            ctx.set_presence(None, OnlineStatus::Online);
            ctx.set_activity(Some(ActivityData::playing(
                data.settings.lock().await.gameserver_address.clone(),
            )));
        }
        serenity::FullEvent::Resume { event: _ } => {
            log::info!("Connection to discord resumed.");
        }
        _ => {}
    }

    Ok(())
}

pub async fn run(settings: Settings, server: Server) -> Result<()> {
    let token = settings.token.clone();

    let options = poise::FrameworkOptions {
        commands: vec![
            info::about(),
            info::status(),
            help::help(),
            owner::quit(),
            owner::admin(),
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
        event_handler: |ctx, event, framework, user_data| {
            Box::pin(event_handler(ctx, event, framework, user_data))
        },
        on_error: |error| Box::pin(on_error(error)),
        pre_command: |ctx| Box::pin(pre_command(ctx)),
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

    let framework = poise::Framework::builder()
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    settings: Mutex::new(settings),
                    state: Mutex::new(state),
                    server: Mutex::new(server),
                })
            })
        })
        .options(options)
        .build();

    serenity::Client::builder(&token, serenity::GatewayIntents::non_privileged())
        .framework(framework)
        .await?
        .start()
        .await?;

    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
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
