use crate::{
    config::ProcliConfig,
    proc::Process,
    ui::{
        debug::DebugWidget,
        process::ProcessWidget,
        state::{Focussable, Mode, UiState},
    },
};
use ratatui::{buffer::Buffer, layout::Rect, macros::*, prelude::*, widgets::*};
use tui_logger::*;

pub struct DashboardWidget<'a> {
    pub ui: &'a UiState,
    pub processes: &'a [Process],
    pub config: &'a ProcliConfig,
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
            DebugWidget { ui: self.ui }.render(panel_rect, buf);
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
                let focussed = matches!(
                    &self.ui.focus,
                    Some(Focussable::Process(i)) if *i == index
                );
                if focussed && matches!(self.ui.mode, Mode::Spotlight) {
                    continue;
                }
                ProcessWidget {
                    process: proc,
                    focussed,
                    ui: self.ui,
                }
                .render(area, buf);
            }
        }

        if matches!(self.ui.mode, Mode::Spotlight)
            && let Some(Focussable::Process(i)) = &self.ui.focus
            && let Some(proc) = self.processes.get(*i)
        {
            ProcessWidget {
                process: proc,
                focussed: true,
                ui: self.ui,
            }
            .render(main_rect.inner(Margin::new(2, 2)), buf);
        }
    }
}
