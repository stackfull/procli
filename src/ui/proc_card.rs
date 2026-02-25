use std::time::Instant;

use rat_widget::focus::{FocusFlag, HasFocus};
use ratatui::{
    macros::{line as rline, span, vertical},
    prelude::*,
    widgets::{Block, BorderType},
};

use crate::{
    proc::process::{Process, ProcessRestart, ProcessState},
    ui::{stat_line::split_stats, theme::Theme},
};

#[derive(Debug)]
pub struct ProcessGrid<'a> {
    pub time: Instant,
    pub theme: Theme,
    pub processes: &'a [Process],
}

#[derive(Debug)]
pub struct ProcessCard<'a> {
    pub time: Instant,
    pub theme: Theme,
    pub process: &'a Process,
}

#[derive(Debug)]
pub struct ProcessGridState {
    pub focus: FocusFlag,
    pub area: Rect,
    pub proc_columns: usize,
    pub proc_rows: usize,
    pub procs: Vec<ProcessCardState>,
}

#[derive(Debug)]
pub struct ProcessCardState {
    pub focus: FocusFlag,
    pub area: Rect,
    pub idx: usize,
}

impl Default for ProcessGridState {
    fn default() -> Self {
        Self {
            focus: FocusFlag::new().with_name("cards"),
            area: Default::default(),
            proc_columns: 2,
            proc_rows: 3,
            procs: Default::default(),
        }
    }
}

impl HasFocus for ProcessGridState {
    fn build(&self, builder: &mut rat_widget::focus::FocusBuilder) {
        let tag = builder.start(self);
        for card in self.procs.as_slice() {
            builder.widget(card);
        }
        builder.end(tag);
    }

    fn focus(&self) -> rat_widget::focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }
}

impl<'a> ProcessCard<'a> {
    pub fn title_line(&self) -> Line<'_> {
        ratatui::macros::line!(
            " SVC ".fg(self.theme.primary),
            self.process.display.clone().fg(self.theme.foreground),
            " "
        )
    }

    pub fn status_indicator(&self) -> Span<'_> {
        match self.process.state {
            ProcessState::Starting => {
                Span::from(self.status_progress_throbber()).fg(self.theme.foreground)
            }
            ProcessState::Running => Span::from(" ● ").fg(self.theme.success),
            ProcessState::Killing(_) => {
                Span::from(self.status_progress_throbber()).fg(self.theme.warning)
            }
            ProcessState::Stopped(ProcessRestart::NoRestart, _) => {
                Span::from(" ○ ").fg(self.theme.error)
            }
            ProcessState::Stopped(_, _) => Span::from(" ⟳ ").fg(self.theme.error),
        }
    }

    pub fn updown_indicator(&self) -> Span<'_> {
        match self.process.state {
            ProcessState::Starting => span!(""),
            ProcessState::Running => span!("↑"),
            ProcessState::Killing(_) => span!("↓"),
            ProcessState::Stopped(_, _) => span!("↓"),
        }
    }

    pub fn uptime(&self) -> String {
        match self.process.last_start {
            Some(then) => {
                let last_stop = self.process.last_stop.unwrap_or(then);
                match self.process.state {
                    ProcessState::Starting => "...".to_string(),
                    ProcessState::Running => {
                        format!("{}s", self.time.duration_since(then).as_secs())
                    }
                    ProcessState::Killing(_) | ProcessState::Stopped(_, _) => {
                        format!("{}s", last_stop.duration_since(then).as_secs())
                    }
                }
            }
            None => "-".to_string(),
        }
    }

    pub fn status_progress_throbber(&self) -> &'static str {
        const FRAMES: [&str; 4] = ["◑", "◒", "◐", "◓"];
        let frame = 1; // TODO: self.ui.step_of_4_in_1_second();
        FRAMES[frame]
    }

    pub fn signal_throbber(&self) -> &'static str {
        const FRAMES: [&str; 4] = ["ᔐ", "ᯇ", "ᔑ", "ᯇ"];
        let frame = 1; // TODO: self.ui.step_of_4_in_1_second();
        FRAMES[frame]
    }
}

impl HasFocus for ProcessCardState {
    fn build(&self, builder: &mut rat_widget::focus::FocusBuilder) {
        builder.leaf_widget(self);
    }

    fn focus(&self) -> rat_widget::focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }
}

impl<'a> StatefulWidget for ProcessGrid<'a> {
    type State = ProcessGridState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let col_constraints = (0..state.proc_columns).map(|_| Constraint::Fill(1));
        let row_constraints = (0..state.proc_rows).map(|_| Constraint::Length(5));
        let horizontal = Layout::horizontal(col_constraints)
            .spacing(1)
            .horizontal_margin(1);
        let vertical = Layout::vertical(row_constraints).spacing(1).margin(1);

        let rows = vertical.split(area);
        let mut cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());
        for (index, card_state) in state.procs.iter_mut().enumerate() {
            let proc = self.processes.get(index);
            if let Some(area) = cells.next()
                && let Some(proc) = proc
            {
                ProcessCard {
                    time: self.time,
                    theme: self.theme,
                    process: proc,
                }
                .render(area, buf, card_state);
            }
        }
    }
}

impl<'a> StatefulWidget for ProcessCard<'a> {
    type State = ProcessCardState;

    /// Render the smaller card version of the process widget.
    ///
    /// ```"not rust"
    /// ╭ SVC Dummy Service 1 ─ ● ───────────────────────────────────────╮
    /// │ __________________________________________█    CPU:     0.0%   │
    /// │ __________________________________________▇▇▇▇ RAM:    16.3MB  │
    /// │                                                                │
    /// ╰────────────────────────────────────────────────────────────────╯
    /// ```
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let status = self.status_indicator();
        let updown = self.updown_indicator();
        let live = !self.process.stats.is_empty();
        let border_color = match state.is_focused() {
            true => self.theme.accent,
            false => self.theme.primary_background,
        };
        let title = self.title_line();
        let mut border = Block::bordered()
            .title_top(title)
            .title_top(status)
            .title_bottom(rline![" ", updown, " ", self.uptime(), " "].right_aligned())
            .border_style(Style::default().bg(self.theme.surface).fg(border_color))
            .bg(self.theme.surface)
            .border_type(BorderType::Rounded);
        let inner = border.inner(area);
        if live {
            border = border.title_top(self.signal_throbber());
        }
        border.render(area, buf);

        if live {
            let (cpu, ram) = split_stats(
                &self.process.stats,
                &self.process.stats_max,
                self.time,
                self.theme,
            );
            let [top, middle, _] = vertical![==1,==1, ==1].areas(inner);
            cpu.render(top, buf);
            ram.render(middle, buf);
        } else {
            let text = Text::from("No Stats Yet");
            let area = inner.centered(
                Constraint::Length(text.width() as u16),
                Constraint::Length(1),
            );
            text.render(area, buf);
        }
    }
}
