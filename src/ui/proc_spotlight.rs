use std::ffi::OsStr;

use crate::{
    proc::process::{ProcessRestart, ProcessState},
    ui::{
        proc_card::{ProcessCard, ProcessCardState},
        stat_line::split_stats,
    },
};
use ratatui::{
    buffer::Buffer, layout::Rect, macros::line as rline, macros::*, prelude::*, style::Stylize,
    widgets::*,
};

#[derive(Debug)]
pub struct ProcessSpotlight<'a> {
    pub card: ProcessCard<'a>,
}

impl ProcessSpotlight<'_> {
    fn field_line<'a, T: Into<Span<'a>>>(&self, label: &'a str, value: T) -> Line<'a> {
        let mut s: Span = value.into();
        if s.style.fg.is_none() {
            s = s.fg(self.card.theme.foreground);
        }
        rline!(label.fg(self.card.theme.primary), s)
    }

    fn command_string(&self) -> String {
        let cmd = &self.card.process.cmd.as_std();
        let args = cmd.get_args().collect::<Vec<_>>().join(OsStr::new(" "));
        format!("{} {}", cmd.get_program().display(), args.display())
    }

    fn restart_policy_string(&self) -> String {
        if self.card.process.restart_policy.enabled {
            format!(
                "Enabled: max: {}, cooldown={}s",
                self.card.process.restart_policy.max_restarts,
                self.card.process.restart_policy.cooloff
            )
        } else {
            "No Restart".to_string()
        }
    }

    fn process_state<'a>(&self) -> Span<'a> {
        match &self.card.process.state {
            ProcessState::Starting => span!(self.card.theme.warning; "Starting"),
            ProcessState::Running => span!(self.card.theme.success; "Running"),
            ProcessState::Killing(_) => span!(self.card.theme.warning; "Killing"),
            ProcessState::Stopped(r, e) => {
                let restart = match r {
                    ProcessRestart::NoRestart => "No Restart".to_string(),
                    ProcessRestart::RestartAt(target) => {
                        format!(
                            "Restart in {}",
                            target.duration_since(self.card.time).as_secs()
                        )
                    }
                };
                span!(self.card.theme.error; "Stopped ({}), {}", e.code().unwrap_or(-1), restart)
            }
        }
    }
}

impl<'a> StatefulWidget for ProcessSpotlight<'a> {
    type State = ProcessCardState;

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
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        Clear.render(area, buf);
        let live = !self.card.process.stats.is_empty();
        let mut border = Block::bordered()
            .title(self.card.title_line())
            .border_style(
                Style::default()
                    .bg(self.card.theme.surface)
                    .fg(self.card.theme.accent),
            )
            .bg(self.card.theme.surface)
            .border_type(BorderType::Rounded);
        if live {
            border = border.title_top(self.card.signal_throbber());
        }
        let inner = border.inner(area);
        border.render(area, buf);
        let inner = inner.inner(Margin::new(1, 1));
        let [info, stats] = vertical![>=8, *=1].areas(inner);
        let [definition, _, status] = horizontal![==2/3, ==2, ==1/3].areas(info);
        let cmd_str = self.command_string();
        let dir = match &self.card.process.cmd.as_std().get_current_dir() {
            Some(dir) => dir.display().to_string(),
            None => ".".to_string(),
        };
        let restart_policy = self.restart_policy_string();

        let definition_text = text!(
            self.field_line("Name: ", &self.card.process.name),
            self.field_line("Command: ", &cmd_str),
            self.field_line("Directory: ", &dir),
            self.field_line("Restart Policy: ", &restart_policy),
        );
        let cpu = self
            .card
            .process
            .stats
            .last()
            .map(|s| format!("{:.1}%", s.cpu_percent))
            .unwrap_or_else(|| "-".to_string());
        let ram = self
            .card
            .process
            .stats
            .last()
            .map(|s| format!("{:.1}MB", s.memory_mb))
            .unwrap_or_else(|| "-".to_string());
        definition_text.render(definition, buf);
        let status_text = text!(
            self.field_line("State: ", self.process_state()),
            self.field_line("Restarts: ", self.card.process.restarts.to_string()),
            self.field_line("CPU: ", cpu),
            self.field_line("RAM: ", ram),
            self.field_line("Uptime: ", self.card.uptime())
        );
        status_text.render(status, buf);
        let (_cpu, ram) = split_stats(
            &self.card.process.stats,
            &self.card.process.stats_max,
            self.card.time,
            self.card.theme,
        );
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
        let max_ram = 1.2 * self.card.process.stats_max.memory_mb as f64;
        let ram_dataset = Dataset::default()
            .name("RAM")
            .marker(symbols::Marker::Dot)
            .graph_type(GraphType::Line)
            .style(
                Style::default()
                    .bg(self.card.theme.surface)
                    .fg(self.card.theme.secondary),
            )
            .data(&ram_data);
        let base_style = Style::default()
            .bg(self.card.theme.surface)
            .fg(self.card.theme.foreground);
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
}
