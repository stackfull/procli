use std::ffi::OsStr;

use crate::{
    proc::{Process, ProcessRestart, ProcessState},
    ui::{
        stat_line::split_stats,
        state::{Mode, UiState},
    },
};
use ratatui::{
    buffer::Buffer, layout::Rect, macros::line as rline, macros::*, prelude::*, style::Stylize,
    widgets::*,
};

pub struct ProcessWidget<'a> {
    pub process: &'a Process,
    pub focussed: bool,
    pub ui: &'a UiState,
}

impl ProcessWidget<'_> {
    /// Render the smaller card version of the process widget.
    ///
    /// ```"not rust"
    /// ╭ SVC Dummy Service 1 ─ ● ───────────────────────────────────────╮
    /// │ __________________________________________█    CPU:     0.0%   │
    /// │ __________________________________________▇▇▇▇ RAM:    16.3MB  │
    /// │                                                                │
    /// ╰────────────────────────────────────────────────────────────────╯
    /// ```
    fn render_card(&self, area: Rect, buf: &mut Buffer) {
        let status = self.status_indicator();
        let updown = self.updown_indicator();
        let live = !self.process.stats.is_empty();
        let border_color = match self.focussed {
            true => self.ui.theme.accent,
            false => self.ui.theme.primary_background,
        };
        let title = self.title_line();
        let mut border = Block::bordered()
            .title_top(title)
            .title_top(status)
            .title_bottom(rline![" ", updown, " ", self.uptime(), " "].right_aligned())
            .border_style(Style::default().bg(self.ui.theme.surface).fg(border_color))
            .bg(self.ui.theme.surface)
            .border_type(BorderType::Rounded);
        let inner = border.inner(area);
        if live {
            border = border.title_top(self.signal_throbber());
        }
        border.render(area, buf);

        if live {
            let (cpu, ram) = split_stats(self.ui, &self.process.stats, &self.process.stats_max);
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

    /// Render the larger modal version of the process widget.
    ///
    /// ```"not rust"
    /// ╭ SVC Dummy Service 1 ─ ● ────────────╮
    /// │ Info                   Status       │
    /// │                                     │
    /// │ Chart                               │
    /// │                                     │
    /// ╰─────────────────────────────────────╯
    /// ```
    ///
    fn render_modal(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        let live = !self.process.stats.is_empty();
        let mut border = Block::bordered()
            .title(self.title_line())
            .border_style(
                Style::default()
                    .bg(self.ui.theme.surface)
                    .fg(self.ui.theme.accent),
            )
            .bg(self.ui.theme.surface)
            .border_type(BorderType::Rounded);
        if live {
            border = border.title_top(self.signal_throbber());
        }
        let inner = border.inner(area);
        border.render(area, buf);
        let inner = inner.inner(Margin::new(1, 1));
        let [info, stats] = vertical![>=8, *=1].areas(inner);
        let [definition, _, status] = horizontal![==2/3, ==2, ==1/3].areas(info);
        let cmd_str = self.command_string();
        let dir = match &self.process.cmd.as_std().get_current_dir() {
            Some(dir) => dir.display().to_string(),
            None => ".".to_string(),
        };
        let restart_policy = self.restart_policy_string();

        let definition_text = text!(
            self.field_line("Name: ", &self.process.name),
            self.field_line("Command: ", &cmd_str),
            self.field_line("Directory: ", &dir),
            self.field_line("Restart Policy: ", &restart_policy),
        );
        let cpu = self
            .process
            .stats
            .last()
            .map(|s| format!("{:.1}%", s.cpu_percent))
            .unwrap_or_else(|| "-".to_string());
        let ram = self
            .process
            .stats
            .last()
            .map(|s| format!("{:.1}MB", s.memory_mb))
            .unwrap_or_else(|| "-".to_string());
        definition_text.render(definition, buf);
        let status_text = text!(
            self.field_line("State: ", self.process_state()),
            self.field_line("Restarts: ", self.process.restarts.to_string()),
            self.field_line("CPU: ", cpu),
            self.field_line("RAM: ", ram),
            self.field_line("Uptime: ", self.uptime())
        );
        status_text.render(status, buf);
        let (_cpu, ram) = split_stats(self.ui, &self.process.stats, &self.process.stats_max);
        // let cpu_data = cpu.data();
        // let cpu_dataset = Dataset::default()
        //     .name("cpu")
        //     .marker(symbols::Marker::Braille)
        //     .graph_type(GraphType::Line)
        //     .style(
        //         Style::default()
        //             .bg(self.ui.theme.surface)
        //             .fg(self.ui.theme.secondary),
        //     )
        //     .data(&cpu_data);
        // let base_style = Style::default()
        //     .bg(self.ui.theme.surface)
        //     .fg(self.ui.theme.foreground);
        // let x_axis = Axis::default()
        //     .title("Seconds ago")
        //     .style(base_style.clone());
        // let y_axis = Axis::default().title("% CPU").style(base_style.clone());
        // let chart = Chart::new(vec![cpu_dataset]).x_axis(x_axis).y_axis(y_axis);
        // chart.render(stats, buf);
        let ram_data = ram.data();
        let max_ram = 1.2 * self.process.stats_max.memory_mb as f64;
        let ram_dataset = Dataset::default()
            .name("RAM")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(
                Style::default()
                    .bg(self.ui.theme.surface)
                    .fg(self.ui.theme.secondary),
            )
            .data(&ram_data);
        let base_style = Style::default()
            .bg(self.ui.theme.surface)
            .fg(self.ui.theme.foreground);
        let x_axis = Axis::default()
            .title("Seconds ago")
            .style(base_style)
            .bounds([-30.0, 0.0])
            .labels([rline!["30"], rline!["15"], rline!["0"]]);
        let y_axis = Axis::default()
            .title("MB")
            .style(base_style)
            .bounds([0.0, max_ram])
            .labels([
                rline!["0.0"],
                rline![format!("{:.2}", max_ram / 2.0)],
                rline![format!("{:.2}", max_ram)],
            ]);
        let chart = Chart::new(vec![ram_dataset]).x_axis(x_axis).y_axis(y_axis);
        chart.render(stats, buf);
    }

    fn field_line<'a, T: Into<Span<'a>>>(&self, label: &'a str, value: T) -> Line<'a> {
        let mut s: Span = value.into();
        if s.style.fg.is_none() {
            s = s.fg(self.ui.theme.foreground);
        }
        rline!(label.fg(self.ui.theme.primary), s)
    }

    fn command_string(&self) -> String {
        let cmd = &self.process.cmd.as_std();
        let args = cmd.get_args().collect::<Vec<_>>().join(OsStr::new(" "));
        format!("{} {}", cmd.get_program().display(), args.display())
    }

    fn restart_policy_string(&self) -> String {
        if self.process.restart_policy.enabled {
            format!(
                "Enabled: max: {}, cooldown={}s",
                self.process.restart_policy.max_restarts, self.process.restart_policy.cooloff
            )
        } else {
            "No Restart".to_string()
        }
    }

    fn process_state<'a>(&self) -> Span<'a> {
        match &self.process.state {
            ProcessState::Starting => span!(self.ui.theme.warning; "Starting"),
            ProcessState::Running => span!(self.ui.theme.success; "Running"),
            ProcessState::Killing(_) => span!(self.ui.theme.warning; "Killing"),
            ProcessState::Stopped(r, e) => {
                let restart = match r {
                    ProcessRestart::NoRestart => "No Restart".to_string(),
                    ProcessRestart::RestartAt(target) => {
                        format!(
                            "Restart in {}",
                            target.duration_since(self.ui.time).as_secs()
                        )
                    }
                };
                span!(self.ui.theme.error; "Stopped ({}), {}", e.code().unwrap_or(-1), restart)
            }
        }
    }

    fn title_line(&self) -> Line<'_> {
        ratatui::macros::line!(
            " SVC ".fg(self.ui.theme.primary),
            self.process.display.clone().fg(self.ui.theme.foreground),
            " "
        )
    }

    fn status_indicator(&self) -> Span<'_> {
        match self.process.state {
            crate::proc::ProcessState::Starting => {
                Span::from(self.status_progress_throbber()).fg(self.ui.theme.foreground)
            }
            crate::proc::ProcessState::Running => Span::from(" ● ").fg(self.ui.theme.success),
            crate::proc::ProcessState::Killing(_) => {
                Span::from(self.status_progress_throbber()).fg(self.ui.theme.warning)
            }
            crate::proc::ProcessState::Stopped(ProcessRestart::NoRestart, _) => {
                Span::from(" ○ ").fg(self.ui.theme.error)
            }
            crate::proc::ProcessState::Stopped(_, _) => Span::from(" ⟳ ").fg(self.ui.theme.error),
        }
    }

    fn updown_indicator(&self) -> Span<'_> {
        match self.process.state {
            ProcessState::Starting => span!(""),
            ProcessState::Running => span!("↑"),
            ProcessState::Killing(_) => span!("↓"),
            ProcessState::Stopped(_, _) => span!("↓"),
        }
    }

    fn uptime(&self) -> String {
        match self.process.last_start {
            Some(then) => {
                let last_stop = self.process.last_stop.unwrap_or(then);
                match self.process.state {
                    ProcessState::Starting => "...".to_string(),
                    ProcessState::Running => {
                        format!("{}s", self.ui.time.duration_since(then).as_secs())
                    }
                    ProcessState::Killing(_) | ProcessState::Stopped(_, _) => {
                        format!("{}s", last_stop.duration_since(then).as_secs())
                    }
                }
            }
            None => "-".to_string(),
        }
    }

    fn status_progress_throbber(&self) -> &'static str {
        const FRAMES: [&str; 4] = ["◑", "◒", "◐", "◓"];
        let frame = self.ui.step_of_4_in_1_second();
        FRAMES[frame]
    }

    fn signal_throbber(&self) -> &'static str {
        const FRAMES: [&str; 4] = ["ᔐ", "ᯇ", "ᔑ", "ᯇ"];
        let frame = self.ui.step_of_4_in_1_second();
        FRAMES[frame]
    }
}

impl<'a> Widget for ProcessWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.focussed && matches!(self.ui.mode, Mode::Spotlight) {
            self.render_modal(area, buf);
        } else {
            self.render_card(area, buf);
        }
    }
}
