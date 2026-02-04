use ratatui::{prelude::*, widgets::*};

use crate::ui::state::{Focussable, UiState};

pub struct DebugWidget<'a> {
    pub ui: &'a UiState,
}

impl Widget for DebugWidget<'_> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let debug = &self.ui;
        let border_color = match self.ui.focus {
            Some(Focussable::Debug) => self.ui.theme.accent,
            _ => self.ui.theme.foreground,
        };
        let panel_style = Style::default()
            .bg(self.ui.theme.surface)
            .fg(self.ui.theme.foreground);
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
        paragraph.render(area, buf);
    }
}
