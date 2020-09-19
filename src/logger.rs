use crate::Result;
use fern::colors::{Color, ColoredLevelConfig};

/// Setup logging.
pub fn init() -> Result<()> {
    let log_path = std::env::var("BOT_LOGS").unwrap_or_else(|_| "bot.log".to_string());

    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Cyan)
        .debug(Color::Green)
        .trace(Color::BrightBlack);

    let base = fern::Dispatch::new();

    let file_cfg = fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}:{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record
                    .line()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| "X".to_string()),
                record.level(),
                message
            ))
        })
        .chain(fern::log_file(log_path)?);

    let mut stdout_cfg =
        fern::Dispatch::new()
            .level(log::LevelFilter::Info)
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "[{}:{}][{}] {}",
                    record.target(),
                    record
                        .line()
                        .map(|x| x.to_string())
                        .unwrap_or_else(|| "X".to_string()),
                    colors.color(record.level()),
                    message
                ))
            });

    stdout_cfg = stdout_cfg.chain(std::io::stdout());

    base.chain(file_cfg).chain(stdout_cfg).apply()?;

    Ok(())
}
