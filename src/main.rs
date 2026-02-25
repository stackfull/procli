use clap::{Parser, Subcommand};
use color_eyre::eyre::OptionExt;
use log::*;
use rat_salsa::{
    Control, RunConfig, TermInit,
    poll::{PollCrossterm, PollRendered, PollTasks, PollTimers, PollTokio},
    run_tui,
};
use ratatui::prelude::{Buffer, Rect, StatefulWidget};
use std::path::PathBuf;
// use tui_logger::{
//     TuiLoggerFile, TuiLoggerLevelOutput, init_logger, set_default_level, set_log_file,
// };

use crate::{app::App, config::ConfigManager, event::AppEvent, ui::state::UiState};

pub mod app;
pub mod config;
pub mod event;
pub mod logs;
pub mod proc;
pub mod resample;
pub mod ui;

/// Proccess management CLI for dev systems.
///
/// Run multiple processes (services, stubs and k6 scenarios) and monitor the output.
/// Provide a config file describing the processes.
///
#[derive(Parser, Debug)]
#[command()]
struct Cli {
    #[arg(short, long, value_name = "FILE", default_value = config::DEFAULT_FILE)]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run all processes and monitor
    Run,
    /// Validate the configuration file
    Validate,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    match &cli.command {
        Commands::Validate => validate(cli),
        Commands::Run => run(cli),
    }
}

/// Utility command to parse and print out the config file.
/// TODO: hash out anything that looks like a secret.
fn validate(cli: Cli) -> color_eyre::Result<()> {
    let cm = ConfigManager::new(cli.config.clone())?;
    println!(
        "Configuration from {}: {:#?}",
        cli.config.to_str().ok_or_eyre("bad config file location")?,
        cm.current()
    );
    Ok(())
}

fn run(cli: Cli) -> color_eyre::Result<()> {
    // Note that the app initialises the logger. Don't make logs before it's configured.
    let mut app = App::new(cli.config)?;
    info!("App created {app:#?}");
    let mut ui_state: UiState = UiState::default();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    info!("Runtime Built {rt:#?}");

    rt.block_on(async {
        println!("Hello world");
    });

    run_tui(
        |state: &mut UiState, app: &mut App| {
            // info!("Init...");
            rt.block_on(|| async { app.init(state).await })
            // Ok(())
        },
        |area: Rect, buf: &mut Buffer, state: &mut UiState, app: &mut App| {
            app.render(area, buf, state);
            Ok(())
        },
        |event: &AppEvent, state: &mut UiState, app: &mut App| {
            info!("Event: {event:#?}");
            let ctrl = app.handle_event(event, state)?;
            Ok(ctrl)
        },
        |report: color_eyre::Report, _state: &mut UiState, _app: &mut App| {
            error!("Bad things {:#?}", report);
            Ok(Control::Changed)
        },
        &mut app,
        &mut ui_state,
        RunConfig::inline(10, false)?
            // .term_init(TermInit::default().manual = true)
            .poll(PollCrossterm)
            .poll(PollRendered)
            .poll(PollTasks::default())
            .poll(PollTokio::new(rt)),
    )?;

    info!("TUI Completed");
    Ok(())
}
