use std::{path::PathBuf, time::Instant};

use crate::{
    config::ConfigManager,
    event::AppEvent,
    logs::LogManager,
    proc::manager::ProcessManager,
    ui::{
        debug::DebugWidget,
        proc_card::{ProcessCard, ProcessGrid},
        proc_spotlight::ProcessSpotlight,
        state::UiState,
        theme::Theme,
    },
};
use color_eyre::eyre::Result;
use log::*;
use rat_salsa::{Control, SalsaAppContext, SalsaContext};
use rat_widget::{
    event::{HandleEvent, Regular, ct_event, event_flow},
    focus::{FocusBuilder, HasFocus},
};
use ratatui::{
    macros::*,
    prelude::*,
    widgets::{Block, Clear},
};

/// The main UI mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// All processes and logs
    Dashboard,
    /// Spotlight a single process
    Spotlight,
    /// Large log split view
    Logs,
}

/// The App contains the global model state for procli.
/// Every widget has access to this to get at the process data, config data etc.
#[derive(Debug)]
pub struct App {
    pub time: Instant,
    pub theme: Theme,
    pub mode: Mode,
    pub debug: bool,
    pub config: ConfigManager,
    pub proc: ProcessManager,
    pub ctx: SalsaAppContext<AppEvent, color_eyre::Report>,
}

impl SalsaContext<AppEvent, color_eyre::Report> for App {
    fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<AppEvent, color_eyre::Report>) {
        self.ctx = app_ctx;
    }

    fn salsa_ctx(&self) -> &SalsaAppContext<AppEvent, color_eyre::Report> {
        &self.ctx
    }
}

impl App {
    pub fn new(config_path: PathBuf) -> Result<Self> {
        let app = Self {
            time: Instant::now(),
            theme: Theme::dark(),
            mode: Mode::Dashboard,
            debug: false,
            config: ConfigManager::new(config_path)?,
            proc: ProcessManager::new(),
            ctx: Default::default(),
        };
        LogManager::configure(app.config.current().logging)?;
        Ok(app)
    }

    pub async fn init(&mut self, state: &mut UiState) -> color_eyre::Result<()> {
        self.ctx.set_focus(FocusBuilder::build_for(state));
        self.focus().first();
        self.start()?;
        Ok(())
    }

    pub fn handle_event(
        &mut self,
        event: &AppEvent,
        state: &mut UiState,
    ) -> Result<Control<AppEvent>> {
        match event {
            // TODO: reimplement this in salsa
            // Event::Tick => {
            //     self.tick();
            //     Control::Continue
            // }
            AppEvent::Crossterm(cev) => {
                // First give the focussed widget a chance.
                event_flow!(self.focus_mut().handle(cev, Regular));
                // Otherwise handle app-level key bindings.
                match cev {
                    ct_event!(key press 'q') => {
                        self.queue_event(AppEvent::Quit);
                    }
                    ct_event!(key press CONTROL-'c') => self.queue_event(AppEvent::Quit),
                    ct_event!(key press 'r') => self.queue_event(AppEvent::Reload),
                    ct_event!(key press 'd') => self.queue_event(AppEvent::ToggleDebug),
                    ct_event!(keycode press Enter) => self.queue_event(AppEvent::ToggleSpotlight),
                    _ => return Ok(Control::Continue),
                }
                Ok(Control::Unchanged)
            }
            // _ => match state.focus {
            //     Some(Focussable::Logs) => {
            //         state.logger_state.transition(match key_event.code {
            //             KeyCode::Esc => TuiWidgetEvent::EscapeKey,
            //             KeyCode::PageUp => TuiWidgetEvent::PrevPageKey,
            //             KeyCode::PageDown => TuiWidgetEvent::NextPageKey,
            //             KeyCode::Left => TuiWidgetEvent::LeftKey,
            //             KeyCode::Right => TuiWidgetEvent::RightKey,
            //             KeyCode::Up => TuiWidgetEvent::UpKey,
            //             KeyCode::Down => TuiWidgetEvent::DownKey,
            //             KeyCode::Char(' ') => TuiWidgetEvent::SpaceKey,
            //             KeyCode::Char('h') => TuiWidgetEvent::HideKey,
            //             KeyCode::Char('f') => TuiWidgetEvent::FocusKey,
            //             KeyCode::Char('+') => TuiWidgetEvent::PlusKey,
            //             KeyCode::Char('-') => TuiWidgetEvent::MinusKey,
            //             _ => return Ok(()),
            //         });
            //     }
            //     Some(Focussable::Process(_)) => {}
            //     Some(Focussable::Debug) => match key_event.code {
            //         KeyCode::Char('k') | KeyCode::Up => {
            //             state.debug_vertical_scroll.prev();
            //         }
            //         KeyCode::Char('j') | KeyCode::Down => {
            //             state.debug_vertical_scroll.next();
            //         }
            //         KeyCode::Char('K') | KeyCode::PageUp => {
            //             page_up(&mut state.debug_vertical_scroll)
            //         }
            //         KeyCode::Char('J') | KeyCode::PageDown => {
            //             page_down(&mut state.debug_vertical_scroll)
            //         }
            //         KeyCode::Home => {
            //             state.debug_vertical_scroll.first();
            //         }
            //         KeyCode::End => {
            //             state.debug_vertical_scroll.last();
            //         }

            //         _ => return Ok(()),
            //     }
            //     _ => Control::Continue,
            // },
            AppEvent::Rendered => {
                self.set_focus(FocusBuilder::rebuild_for(state, self.take_focus()));
                Ok(Control::Continue)
            }

            AppEvent::Reload => {
                self.reload_config();
                Ok(Control::Changed)
            }
            AppEvent::Quit => Ok(Control::Quit),
            AppEvent::ProcessDied(id, status) => {
                self.proc.process_died(*id, *status);
                Ok(Control::Changed)
            }
            AppEvent::StatsRefresh => {
                self.proc.tick();
                Ok(Control::Changed)
            }
            AppEvent::ToggleSpotlight => {
                if self.mode == Mode::Spotlight {
                    self.mode = Mode::Dashboard;
                } else {
                    self.mode = Mode::Spotlight;
                }
                Ok(Control::Changed)
            }
            AppEvent::ToggleDebug => {
                self.debug = !self.debug;
                Ok(Control::Changed)
            }
            _ => Ok(Control::Continue),
        }
    }

    fn reload_config(&mut self) {
        debug!(target:"App", "Reload!");
        match self.config.reload() {
            Ok(_) => {
                if let Some(e) = self.start().err() {
                    error!(target: "App", "{}", e);
                }
            }
            Err(e) => error!(target: "App", "{}", e),
        }
    }

    /// Start services, stubs, and agents from the given configuration.
    /// Changes to the service lineup use the names as unique keys but
    /// let the process manager decide whether to restart or not.
    pub fn start(&mut self) -> Result<()> {
        let config = &self.config.current();
        let removals: Vec<String> = self
            .proc
            .processes
            .iter()
            .filter(|proc| config.contains(&proc.name))
            .map(|proc| proc.name.clone())
            .collect();
        for name in removals {
            debug!("Stop process {name}");
            self.proc.remove(&name)?;
        }
        for stub in config.stubs.iter() {
            debug!("Start stub {}", stub.name);
            self.proc.upsert(stub)?;
            self.spawn_async(async { Ok(Control::Event(AppEvent::Tick)) });
        }
        for svc in config.services.iter() {
            debug!("Start service {}", svc.name);
            self.proc.upsert(svc)?;
        }
        for agent in config.agents.iter() {
            debug!("Start agent {}", agent.name);
        }

        // Event this:
        // state.update_procs(self.proc.processes.len());
        Ok(())
    }
}

// fn page_up(state: &mut ScrollbarState) {
//     let pos = state.get_position().saturating_sub(20);
//     *state = state.position(pos);
// }

// fn page_down(state: &mut ScrollbarState) {
//     let pos = state.get_position().saturating_add(20);
//     *state = state.position(pos);
// }

impl StatefulWidget for &mut App {
    type State = UiState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);
        let [window_rect, log_rect] = vertical![>=5, ==10].areas(area);

        // let panel_style = Style::default()
        //     .bg(self.theme.surface)
        //     .fg(self.theme.foreground);

        let main_rect = if self.debug {
            let [main_rect, panel_rect] = horizontal![>=5, >=30].areas(window_rect);
            DebugWidget {
                theme: self.theme,
                processes: self.proc.processes.as_slice(),
                config: &self.config.current(),
            }
            .render(panel_rect, buf, &mut state.debug);
            main_rect
        } else {
            window_rect
        };

        // let border_color = match self.focus {
        //     Some(Focussable::Logs) => self.ui.theme.accent,
        //     _ => self.ui.theme.foreground,
        // };
        // TuiLoggerSmartWidget::default()
        //     .style_error(panel_style.fg(self.ui.theme.error))
        //     .style_debug(panel_style)
        //     .style_warn(panel_style.fg(self.ui.theme.warning))
        //     .style_trace(panel_style)
        //     .style_info(panel_style)
        //     .style(panel_style)
        //     .border_style(panel_style.fg(border_color))
        //     .output_separator(':')
        //     .output_timestamp(Some("%H:%M:%S".to_string()))
        //     .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        //     .output_target(true)
        //     .output_file(true)
        //     .output_line(true)
        //     .state(&self.ui.logger_state)
        //     // .block(Block::bordered().title("Logs"))
        //     .render(log_rect, buf); // TuiLoggerSmartWidget::default()

        let main_style = Style::default()
            .bg(self.theme.background)
            .fg(self.theme.foreground);
        Block::new().style(main_style).render(main_rect, buf);

        ProcessGrid {
            time: self.time,
            theme: self.theme,
            processes: self.proc.processes.as_slice(),
        }
        .render(main_rect, buf, &mut state.cards);

        if matches!(self.mode, Mode::Spotlight) {
            // match_focus!()
            if let Some(focussed) = state.cards.procs.iter_mut().find(|x| x.is_focused()) {
                if let Some(proc) = self.proc.processes.get(focussed.idx) {
                    ProcessSpotlight {
                        card: ProcessCard {
                            time: self.time,
                            theme: self.theme,
                            process: proc,
                        },
                    }
                    .render(main_rect.inner(Margin::new(2, 2)), buf, focussed);
                }
            }
        }
    }
}
