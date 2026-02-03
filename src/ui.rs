use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{
    config::PratConfig,
    event::TICK_FPS,
    proc::{Process, ProcessRestart, ProcessStats},
    theme::Theme,
};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    macros::*,
    prelude::*,
    style::Stylize,
    widgets::*,
};
use tui_logger::*;

pub struct DashboardWidget<'a> {
    pub ui: &'a UiState,
    pub processes: &'a [Process],
    pub config: &'a PratConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focussable {
    Process(usize),
    Logs,
    Debug,
}

pub struct UiState {
    tick: f64,
    time: Instant,
    proc_columns: usize,
    proc_rows: usize,
    theme: Theme,
    procs: usize,
    pub focus: Option<Focussable>,
    debug: bool,
    pub logger_state: TuiWidgetState,
}

impl Debug for UiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiState")
            .field("tick", &self.tick)
            .field("time", &self.time)
            .field("proc_columns", &self.proc_columns)
            .field("proc_rows", &self.proc_rows)
            .field("procs", &self.procs)
            .field("focus", &self.focus)
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
            procs: 0,
            theme: Theme::dark(),
            focus: None,
            debug: false,
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

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
        if !self.debug
            && let Some(Focussable::Debug) = &self.focus
        {
            self.focus = Some(Focussable::Process(0));
        }
    }

    pub fn focus_next(&mut self) {
        self.focus = match &self.focus {
            None => Some(Focussable::Process(0)),
            Some(Focussable::Process(i)) => {
                if i + 1 < self.procs {
                    Some(Focussable::Process(i + 1))
                } else {
                    Some(Focussable::Logs)
                }
            }
            Some(Focussable::Logs) => {
                if self.debug {
                    Some(Focussable::Debug)
                } else {
                    Some(Focussable::Process(0))
                }
            }
            Some(Focussable::Debug) => Some(Focussable::Process(0)),
        }
    }

    pub fn update_procs(&mut self, count: usize) {
        self.procs = count;
        if let Some(Focussable::Process(idx)) = &self.focus
            && *idx >= self.procs
        {
            self.focus = Some(if self.procs == 0 {
                Focussable::Logs
            } else {
                Focussable::Process(self.procs - 1)
            });
        }
    }
}

impl<'a> Widget for &mut DashboardWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let [window_rect, log_rect] = vertical![>=5, ==10].areas(area);

        let panel_style = Style::default()
            .bg(self.ui.theme.surface)
            .fg(self.ui.theme.foreground);

        let main_rect = if self.ui.debug {
            let [main_rect, panel_rect] = horizontal![>=5, >=30].areas(window_rect);
            let debug = (&self.ui, &self.processes);
            let border_color = match self.ui.focus {
                Some(Focussable::Debug) => self.ui.theme.accent,
                _ => self.ui.theme.foreground,
            };
            let paragraph = Paragraph::new(format!("{debug:#?}"))
                .block(
                    Block::bordered()
                        .title("Debug")
                        .title_alignment(Alignment::Left)
                        .border_style(Style::default().fg(border_color))
                        .border_type(BorderType::Rounded),
                )
                .alignment(HorizontalAlignment::Left)
                .style(panel_style);
            paragraph.render(panel_rect, buf);
            main_rect
        } else {
            window_rect
        };

        let border_color = match self.ui.focus {
            Some(Focussable::Logs) => self.ui.theme.accent,
            _ => self.ui.theme.foreground,
        };
        TuiLoggerSmartWidget::default()
            .style_error(panel_style.fg(self.ui.theme.error))
            .style_debug(panel_style)
            .style_warn(panel_style.fg(self.ui.theme.warning))
            .style_trace(panel_style)
            .style_info(panel_style)
            .style(panel_style)
            .border_style(panel_style.fg(border_color))
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(true)
            .output_file(true)
            .output_line(true)
            .state(&self.ui.logger_state)
            // .block(Block::bordered().title("Logs"))
            .render(log_rect, buf); // TuiLoggerSmartWidget::default()

        let main_style = Style::default()
            .bg(self.ui.theme.background)
            .fg(self.ui.theme.foreground);
        Block::new().style(main_style).render(main_rect, buf);

        let col_constraints = (0..self.ui.proc_columns).map(|_| Constraint::Fill(1));
        let row_constraints = (0..self.ui.proc_rows).map(|_| Constraint::Length(5));
        let horizontal = Layout::horizontal(col_constraints)
            .spacing(1)
            .horizontal_margin(1);
        let vertical = Layout::vertical(row_constraints).spacing(1).margin(1);

        let rows = vertical.split(main_rect);
        let mut cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());
        for (index, proc) in self.processes.iter().enumerate() {
            if let Some(area) = cells.next() {
                ProcessWidget {
                    process: proc,
                    focussed: matches!(
                        &self.ui.focus,
                        Some(Focussable::Process(i)) if *i == index
                    ),
                    ui: self.ui,
                }
                .render(area, buf);
            }
        }
    }
}

struct ProcessWidget<'a> {
    process: &'a Process,
    focussed: bool,
    ui: &'a UiState,
}

// ["◑", "◒", "◐", "◓"]
// ["ᔐ", "ᯇ", "ᔑ", "ᯇ"]
impl<'a> Widget for ProcessWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status = match self.process.state {
            crate::proc::ProcessState::Starting => Span::from("◐").fg(self.ui.theme.warning),
            crate::proc::ProcessState::Running => Span::from("●").fg(self.ui.theme.success),
            crate::proc::ProcessState::Killing(_) => Span::from("◑").fg(self.ui.theme.warning),
            crate::proc::ProcessState::Stopped(ProcessRestart::NoRestart, _) => {
                Span::from("○").fg(self.ui.theme.error)
            }
            crate::proc::ProcessState::Stopped(_, _) => Span::from("⟳").fg(self.ui.theme.error),
        };
        let border_color = match self.focussed {
            true => self.ui.theme.accent,
            false => self.ui.theme.foreground,
        };
        let border = Block::bordered()
            .title(Span::from(" SVC").fg(self.ui.theme.primary))
            .title(Span::from(self.process.display.clone()).style(self.ui.theme.foreground))
            .title(status)
            .border_style(
                Style::default()
                    .bg(self.ui.theme.surface)
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .bg(self.ui.theme.surface)
            .border_type(BorderType::Rounded);
        let inner = border.inner(area);
        border.render(area, buf);

        if self.process.stats.is_empty() {
            let text = Text::from("No Stats Yet");
            let area = inner.centered(
                Constraint::Length(text.width() as u16),
                Constraint::Length(1),
            );
            text.render(area, buf);
        } else {
            let (cpu, ram) = split_stats(self.ui, &self.process.stats, &self.process.stats_max);
            let [top, middle, _] = vertical![==1,==1, ==1].areas(inner);
            cpu.render(top, buf);
            ram.render(middle, buf);
        }
    }
}

#[derive(Debug)]
pub struct SingleStat<'a> {
    name: String,
    unit: String,
    history: Vec<f32>,
    max: f32,
    timestamps: Vec<Instant>,
    ui: &'a UiState,
}

fn split_stats<'a>(
    ui: &'a UiState,
    stats: &[ProcessStats],
    max_stats: &ProcessStats,
) -> (SingleStat<'a>, SingleStat<'a>) {
    let timestamps: Vec<Instant> = stats.iter().map(|s| s.timestamp).collect();
    let cpu_history = SingleStat {
        name: "CPU".to_string(),
        unit: "%".to_string(),
        history: stats.iter().map(|s| s.cpu_percent).collect(),
        max: max_stats.cpu_percent,
        timestamps: timestamps.clone(),
        ui,
    };
    let mem_history = SingleStat {
        name: "RAM".to_string(),
        unit: "MB".to_string(),
        history: stats.iter().map(|s| s.memory_mb).collect(),
        max: max_stats.memory_mb,
        timestamps,
        ui,
    };
    (cpu_history, mem_history)
}

impl<'a> Widget for &SingleStat<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [_, history, _, label, current, _] =
            horizontal![==1, *=1, ==1, ==6, ==8, ==2].areas(area);
        Text::from(self.name.clone() + ":").render(label, buf);
        ratatui::macros::line![
            span![format!("{:.1}", self.history.last().unwrap_or(&0.0))],
            span![format!("{:<2}", self.unit.clone())].fg(self.ui.theme.primary_background)
        ]
        .alignment(Alignment::Right)
        .render(current, buf);
        let resampled: Vec<Option<u64>> = crate::resample::resample(
            &self.history,
            &self.timestamps,
            self.ui.time - Duration::from_secs(120),
            self.ui.time,
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
            .fg(self.ui.theme.primary)
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
