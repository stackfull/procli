use std::{
    fmt::Debug,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::{
    config::{ConfigManager, PratConfig},
    event::{AppEvent, Event, EventHandler, TICK_FPS},
    proc::{Process, ProcessManager, ProcessRestart, ProcessStats},
    theme::Theme,
};
use color_eyre::eyre::Result;
use log::*;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Alignment, Rect},
    macros::*,
    prelude::*,
    style::Stylize,
    widgets::*,
};
use tui_logger::*;

pub struct App {
    pub running: bool,
    pub events: EventHandler,
    pub config: ConfigManager,
    pub proc: ProcessManager,
    pub ui_state: UiState,
}

pub struct UiState {
    pub logger_state: TuiWidgetState,
    tick: f64,
    time: Instant,
    proc_columns: usize,
    proc_rows: usize,
    theme: Theme,
}

impl Debug for UiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiState")
            .field("tick", &self.tick)
            .field("time", &self.time)
            .field("proc_columns", &self.proc_columns)
            .field("proc_rows", &self.proc_rows)
            .finish()
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            logger_state: TuiWidgetState::new(),
            tick: Default::default(),
            time: Instant::now(),
            proc_columns: 2,
            proc_rows: 3,
            theme: Theme::dark(),
        }
    }
}

impl UiState {
    pub fn tick(&mut self) {
        self.tick += 1.0;
        if self.tick > 2.0 * TICK_FPS {
            self.tick = 0.0;
            self.time = Instant::now();
        }
    }

    pub fn step_of_8_in_1_second(&self) -> usize {
        (self.tick * 8.0 / TICK_FPS) as usize % 8
    }

    pub fn step_of_4_in_1_second(&self) -> usize {
        (self.tick * 4.0 / TICK_FPS) as usize % 4
    }

    pub fn step_of_8_in_2_second(&self) -> usize {
        (self.tick * 4.0 / TICK_FPS) as usize % 8
    }
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
                if let Some(e) = self.start(config).err() {
                    error!(target: "App", "{}", e);
                }
            }
            Err(e) => error!(target: "App", "{}", e),
        }
    }

    /// Start services, stubs, and agents from the given configuration.
    /// Changes to the service lineup use the names as unique keys but
    /// let the process manager decide whether to restart or not.
    fn start(&mut self, config: PratConfig) -> Result<()> {
        for stub in config.stubs {
            debug!("Start stub {}", stub.name);
        }
        for svc in config.services {
            debug!("Start service {}", svc.name);
            self.proc.upsert(&svc)?;
        }
        // TODO: stop services that are no longer in the config
        for agent in config.agents {
            debug!("Start agent {}", agent.name);
        }
        Ok(())
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let [window_rect, log_rect] = vertical![>=5, ==10].areas(area);
        // let [main_rect, panel_rect] = horizontal![>=5, >=30].areas(window_rect);
        let main_rect = window_rect;

        // let debug = (&self.ui_state.time, &self.proc.processes);
        // let paragraph = Paragraph::new(format!("{debug:#?}"))
        //     .block(
        //         Block::bordered()
        //             .title(format!("Debug: {:?}", self.config.file_path))
        //             .title_alignment(Alignment::Left)
        //             .border_type(BorderType::Rounded),
        //     )
        //     .alignment(HorizontalAlignment::Left)
        //     .fg(self.ui_state.theme.foreground)
        //     .bg(self.ui_state.theme.surface);
        // paragraph.render(panel_rect, buf);

        let panel_style = Style::default()
            .bg(self.ui_state.theme.surface)
            .fg(self.ui_state.theme.foreground);
        TuiLoggerSmartWidget::default()
            .style_error(panel_style.clone().fg(self.ui_state.theme.error))
            .style_debug(panel_style.clone())
            .style_warn(panel_style.clone().fg(self.ui_state.theme.warning))
            .style_trace(panel_style.clone())
            .style_info(panel_style.clone())
            .style(panel_style.clone())
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(true)
            .output_line(true)
            .state(&self.ui_state.logger_state)
            // .block(Block::bordered().title("Logs"))
            .render(log_rect, buf); // TuiLoggerSmartWidget::default()

        let main_style = Style::default()
            .bg(self.ui_state.theme.background)
            .fg(self.ui_state.theme.foreground);
        Block::new().style(main_style).render(main_rect, buf);

        let col_constraints = (0..self.ui_state.proc_columns).map(|_| Constraint::Fill(1));
        let row_constraints = (0..self.ui_state.proc_rows).map(|_| Constraint::Length(5));
        let horizontal = Layout::horizontal(col_constraints)
            .spacing(1)
            .horizontal_margin(1);
        let vertical = Layout::vertical(row_constraints).spacing(1).margin(1);

        let rows = vertical.split(main_rect);
        let mut cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());
        for proc in self.proc.processes.iter() {
            if let Some(area) = cells.next() {
                proc.render(area, buf, &mut self.ui_state);
            }
        }
    }
}

impl StatefulWidget for &Process {
    type State = UiState;
    fn render(self, area: Rect, buf: &mut Buffer, ui: &mut Self::State) {
        let status = match self.state {
            crate::proc::ProcessState::Starting => Span::from("◐").fg(ui.theme.warning),
            crate::proc::ProcessState::Running => Span::from("●").fg(ui.theme.success),
            crate::proc::ProcessState::Killing(_) => Span::from("◑").fg(ui.theme.warning),
            crate::proc::ProcessState::Stopped(ProcessRestart::NoRestart, _) => {
                Span::from("○").fg(ui.theme.error)
            }
            crate::proc::ProcessState::Stopped(_, _) => Span::from("⟳").fg(ui.theme.error),
        };
        let border = Block::bordered()
            .title(Span::from(" SVC").fg(ui.theme.primary))
            .title(Span::from(self.display.clone()).style(ui.theme.foreground))
            .title(status)
            .border_style(
                Style::default()
                    .bg(ui.theme.surface)
                    .fg(ui.theme.primary_background)
                    .add_modifier(Modifier::BOLD),
            )
            .bg(ui.theme.surface)
            .border_type(BorderType::Rounded);
        let inner = border.inner(area);
        border.render(area, buf);

        if self.stats.is_empty() {
            let text = Text::from("No Stats Yet");
            let area = inner.centered(
                Constraint::Length(text.width() as u16),
                Constraint::Length(1),
            );
            text.render(area, buf);
        } else {
            let (cpu, ram) = split_stats(&self.stats, &self.stats_max);
            let [top, middle, _] = vertical![==1,==1, ==1].areas(inner);
            cpu.render(top, buf, ui);
            ram.render(middle, buf, ui);
        }
    }
}

#[derive(Debug)]
pub struct SingleStat {
    name: String,
    unit: String,
    history: Vec<f32>,
    max: f32,
    timestamps: Vec<Instant>,
}

fn split_stats(stats: &Vec<ProcessStats>, max_stats: &ProcessStats) -> (SingleStat, SingleStat) {
    let timestamps: Vec<Instant> = stats.iter().map(|s| s.timestamp).collect();
    let cpu_history = SingleStat {
        name: "CPU".to_string(),
        unit: "%".to_string(),
        history: stats.iter().map(|s| s.cpu_percent).collect(),
        max: max_stats.cpu_percent,
        timestamps: timestamps.clone(),
    };
    let mem_history = SingleStat {
        name: "RAM".to_string(),
        unit: "MB".to_string(),
        history: stats.iter().map(|s| s.memory_mb).collect(),
        max: max_stats.memory_mb,
        timestamps: timestamps,
    };
    (cpu_history, mem_history)
}

impl StatefulWidget for &SingleStat {
    type State = UiState;
    fn render(self, area: Rect, buf: &mut Buffer, ui: &mut Self::State) {
        let [_, history, _, label, current, _] =
            horizontal![==1, *=1, ==1, ==6, ==8, ==2].areas(area);
        Text::from(self.name.clone() + ":").render(label, buf);
        ratatui::macros::line![
            span![format!("{:.1}", self.history.last().unwrap_or(&0.0))],
            span![format!("{:<2}", self.unit.clone())].fg(ui.theme.primary_background)
        ]
        .alignment(Alignment::Right)
        .render(current, buf);
        let resampled: Vec<Option<u64>> = crate::resample::resample(
            &self.history,
            &self.timestamps,
            ui.time - Duration::from_secs(120),
            ui.time,
            history.width as usize,
        )
        .iter()
        .map(|o| o.map(|v| v.trunc() as u64))
        .collect();
        // if ui.tick % TICK_FPS < 1.0 {
        //     debug!(
        //         target: "App",
        //         "Resampled {} points for {} over {:?} to {:?}",
        //         self.history.len(),
        //         self.name,
        //         (ui.time - Duration::from_secs(60))..ui.time,
        //         resampled
        //     );
        // }
        Sparkline::default()
            .data(&resampled)
            .max((self.max * 1.1) as u64)
            .absent_value_symbol("_")
            .fg(ui.theme.primary)
            .render(history, buf);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const TICKS_AND_STEPS: [(usize, usize, usize, usize); 13] = [
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (2, 0, 0, 0),
        (1, 0, 1, 0),
        (3, 0, 1, 0),
        (1, 1, 2, 1),
        (3, 1, 2, 1),
        (1, 1, 3, 1),
        (2, 1, 3, 1),
        (1, 2, 4, 2),
        (15, 0, 0, 4),
        (15, 2, 4, 6),
        (15, 0, 0, 0),
    ];

    #[test]
    fn all_the_throbs() {
        let mut t = UiState::default();
        let mut c = 0;
        for (ticks, s4i1, s8i1, s8i2) in TICKS_AND_STEPS {
            for _ in 0..ticks {
                t.tick();
                c += 1;
            }
            assert_eq!(
                t.step_of_4_in_1_second(),
                s4i1,
                "After {} ticks, 4/1 should be {}",
                c,
                s4i1
            );
            assert_eq!(
                t.step_of_8_in_1_second(),
                s8i1,
                "After {} ticks, 8/1 should be {}",
                c,
                s8i1
            );
            assert_eq!(
                t.step_of_8_in_2_second(),
                s8i2,
                "After {} ticks, 8/2 should be {}",
                c,
                s8i2
            );
        }
    }
}
