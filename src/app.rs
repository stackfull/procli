use std::path::PathBuf;

use crate::{
    config::{ConfigManager, PratConfig},
    event::{AppEvent, Event, EventHandler},
    proc::ProcessManager,
};
use color_eyre::eyre::{self, Result};
use log::*;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Alignment, Rect},
    macros::*,
    prelude::*,
    style::{Color, Stylize},
    widgets::*,
};
use tui_logger::*;

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub events: EventHandler,
    pub config: ConfigManager,
    pub proc: ProcessManager,
}

impl App {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let events = EventHandler::new();
        let sender = events.clone_sender();
        Ok(Self {
            running: true,
            events,
            config: ConfigManager::new(config_path, sender)?,
            proc: ProcessManager::default(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        if let Some(err) = self.start(self.config.current()).err() {
            error!(target: "App", "Failed to start: {}", err);
        }
        while self.running {
            terminal.draw(|frame| self.render(frame.area(), frame.buffer_mut()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        // debug!(target:"App", "Crossterm key event {:?}", event);
                        self.handle_key_events(key_event)?
                    }
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Reload => self.reload_config(),
                    AppEvent::Quit => self.quit(),
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Char('r') => self.events.send(AppEvent::Reload),
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    fn tick(&self) {}

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn reload_config(&mut self) {
        debug!(target:"App", "Reload!");
        match self.config.reload() {
            Ok(config) => {
                if let Some(e) = self.start(config).err() {
                    error!(target: "App", "{}", e);
                }
            }
            Err(e) => error!(target: "App", "{}", e),
        }
    }

    fn start(&mut self, config: PratConfig) -> Result<()> {
        for stub in config.stubs {
            debug!("Start stub {}", stub.name);
        }
        for svc in config.services {
            debug!("Start service {}", svc.name);
            self.proc.start(&svc)?;
        }
        for agent in config.agents {
            debug!("Start agent {}", agent.name);
        }
        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [window_rect, log_rect] = vertical![>=5, ==10].areas(area);
        let [main_rect, conf_rect] = horizontal![>=5, ==30].areas(window_rect);

        let block = Block::bordered()
            .title(format!("Config: {:?}", self.config.file_path))
            .title_alignment(Alignment::Left)
            .border_type(BorderType::Rounded);
        let config = self.config.current();
        let paragraph = Paragraph::new(format!("{config:#?}"))
            .block(block)
            .fg(Color::LightYellow)
            .bg(Color::Black)
            .centered();

        paragraph.render(conf_rect, buf);
        // TuiLoggerSmartWidget::default()
        TuiLoggerWidget::default()
            .block(Block::bordered().title("Logs"))
            .render(log_rect, buf);
    }
}
