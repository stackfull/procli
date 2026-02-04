use crate::{
    proc::{Process, ProcessRestart},
    ui::stat_line::split_stats,
    ui::state::UiState,
};
use ratatui::{buffer::Buffer, layout::Rect, macros::*, prelude::*, style::Stylize, widgets::*};

pub struct ProcessWidget<'a> {
    pub process: &'a Process,
    pub focussed: bool,
    pub ui: &'a UiState,
}

// ["◑", "◒", "◐", "◓"]
// ["ᔐ", "ᯇ", "ᔑ", "ᯇ"]
impl<'a> Widget for ProcessWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status = match self.process.state {
            crate::proc::ProcessState::Starting => Span::from(" ◐ ").fg(self.ui.theme.warning),
            crate::proc::ProcessState::Running => Span::from(" ● ").fg(self.ui.theme.success),
            crate::proc::ProcessState::Killing(_) => Span::from(" ◑ ").fg(self.ui.theme.warning),
            crate::proc::ProcessState::Stopped(ProcessRestart::NoRestart, _) => {
                Span::from(" ○ ").fg(self.ui.theme.error)
            }
            crate::proc::ProcessState::Stopped(_, _) => Span::from(" ⟳ ").fg(self.ui.theme.error),
        };
        let border_color = match self.focussed {
            true => self.ui.theme.accent,
            false => self.ui.theme.primary_background,
        };
        let title = ratatui::macros::line!(
            " SVC ".fg(self.ui.theme.primary),
            self.process.display.clone().fg(self.ui.theme.foreground),
            " "
        );
        let border = Block::bordered()
            .title(title)
            .title(status)
            .border_style(Style::default().bg(self.ui.theme.surface).fg(border_color))
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
