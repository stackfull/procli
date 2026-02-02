use std::path::PathBuf;

use crate::{
    config::{ConfigManager, PratConfig},
    event::{AppEvent, Event, EventHandler},
    proc::ProcessManager,
    ui::{DashboardWidget, Focussable, UiState},
};
use color_eyre::eyre::Result;
use log::*;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    prelude::*,
};
use tui_logger::TuiWidgetEvent;

pub struct App {
    pub running: bool,
    pub events: EventHandler,
    pub config: ConfigManager,
    pub proc: ProcessManager,
    pub ui_state: UiState,
}

impl App {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let events = EventHandler::new();
        let sender1 = events.clone_sender();
        let sender2 = events.clone_sender();
        Ok(Self {
            running: true,
            events,
            config: ConfigManager::new(config_path, sender1)?,
            proc: ProcessManager::new(sender2),
            ui_state: UiState::default(),
        })
    }

    /// Run the application's main loop.
    pub async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        if let Some(err) = self.start(&self.config.current()).err() {
            error!(target: "App", "Failed to start: {}", err);
        }
        while self.running {
            terminal.draw(|frame| {
                DashboardWidget {
                    ui: &self.ui_state,
                    processes: &self.proc.processes,
                    config: &self.config.current(),
                }
                .render(frame.area(), frame.buffer_mut())
            })?;

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event)
                        if key_event.kind == crossterm::event::KeyEventKind::Press =>
                    {
                        self.handle_key_events(key_event)?
                    }
                    _ => {}
                },
                Event::App(app_event) => match app_event {
                    AppEvent::Reload => self.reload_config(),
                    AppEvent::Quit => self.quit(),
                    AppEvent::ProcessDied(id, status) => self.proc.process_died(id, status),
                    AppEvent::StatsRefresh => self.proc.tick(),
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> Result<()> {
        match key_event.code {
            KeyCode::Char('q') => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Char('r') => self.events.send(AppEvent::Reload),
            KeyCode::Char('d') => self.ui_state.toggle_debug(),
            KeyCode::Tab => self.ui_state.focus_next(),
            // Other handlers you could add here.
            _ => match self.ui_state.focus {
                Some(Focussable::Logs) => {
                    self.ui_state.logger_state.transition(match key_event.code {
                        KeyCode::Esc => TuiWidgetEvent::EscapeKey,
                        KeyCode::PageUp => TuiWidgetEvent::PrevPageKey,
                        KeyCode::PageDown => TuiWidgetEvent::NextPageKey,
                        KeyCode::Left => TuiWidgetEvent::LeftKey,
                        KeyCode::Right => TuiWidgetEvent::RightKey,
                        KeyCode::Up => TuiWidgetEvent::UpKey,
                        KeyCode::Down => TuiWidgetEvent::DownKey,
                        KeyCode::Char(' ') => TuiWidgetEvent::SpaceKey,
                        KeyCode::Char('h') => TuiWidgetEvent::HideKey,
                        KeyCode::Char('f') => TuiWidgetEvent::FocusKey,
                        KeyCode::Char('+') => TuiWidgetEvent::PlusKey,
                        KeyCode::Char('-') => TuiWidgetEvent::MinusKey,
                        _ => return Ok(()),
                    });
                }
                Some(Focussable::Process(idx)) => {}
                Some(Focussable::Debug) => {}
                None => {}
            },
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    fn tick(&mut self) {
        self.ui_state.tick();
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }

    fn reload_config(&mut self) {
        debug!(target:"App", "Reload!");
        match self.config.reload() {
            Ok(config) => {
                if let Some(e) = self.start(&config).err() {
                    error!(target: "App", "{}", e);
                }
            }
            Err(e) => error!(target: "App", "{}", e),
        }
    }

    /// Start services, stubs, and agents from the given configuration.
    /// Changes to the service lineup use the names as unique keys but
    /// let the process manager decide whether to restart or not.
    fn start(&mut self, config: &PratConfig) -> Result<()> {
        let removals: Vec<String> = self
            .proc
            .processes
            .iter()
            .filter(|proc| config.contains(&proc.name))
            .map(|proc| proc.name.clone())
            .collect();
        for name in removals {
            debug!("Stop service {name}");
            self.proc.remove(&name)?;
        }
        for stub in config.stubs.iter() {
            debug!("Start stub {}", stub.name);
        }
        for svc in config.services.iter() {
            debug!("Start service {}", svc.name);
            self.proc.upsert(&svc)?;
        }
        for agent in config.agents.iter() {
            debug!("Start agent {}", agent.name);
        }

        self.ui_state.update_procs(self.proc.processes.len());
        Ok(())
    }
}
