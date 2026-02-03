use clap::{Parser, Subcommand};
use log::info;
use std::path::PathBuf;
use tui_logger::{
    TuiLoggerFile, TuiLoggerLevelOutput, init_logger, set_default_level, set_log_file,
};

use crate::app::App;

pub mod app;
pub mod config;
pub mod event;
pub mod proc;
pub mod resample;
pub mod theme;
pub mod ui;

#[derive(Parser, Debug)]
#[command(about)]
struct Cli {
    #[arg(short, long, value_name = "FILE", default_value = config::DEFAULT_FILE)]
    config: PathBuf,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run all processes and monitor
    Run,
    /// Validate the configuration file
    Validate,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Validate) => Ok(()),
        Some(Commands::Run) | None => {
            init_logger(tui_logger::LevelFilter::Debug)?;
            let file_options = TuiLoggerFile::new("procli.log")
                .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
                .output_file(false)
                .output_separator(':');
            set_log_file(file_options);
            info!("Logging started");
            let mut app = App::new(cli.config)?;
            set_default_level(tui_logger::LevelFilter::Debug);
            let terminal = ratatui::init();
            let result = app.run(terminal).await;
            ratatui::restore();
            result
        }
    }
}
