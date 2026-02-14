use ratatui::{macros::*, prelude::*, widgets::*};

use crate::{
    config::ProcliConfig,
    proc::process::Process,
    ui::state::{Focussable, UiState},
};

#[derive(Debug)]
pub struct DebugWidget<'a> {
    pub processes: &'a [Process],
    pub config: &'a ProcliConfig,
}

impl StatefulWidget for &DebugWidget<'_> {
    type State = UiState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_color = match state.focus {
            Some(Focussable::Debug) => state.theme.accent,
            _ => state.theme.foreground,
        };
        let panel_style = Style::default()
            .bg(state.theme.surface)
            .fg(state.theme.foreground);

        let debug = &self;
        let s = format!("{debug:#?}");
        let lines: Vec<Line> = s.lines().map(|sl| Line::from(sl)).collect();
        let n_lines = lines.len();
        state.debug_vertical_scroll = state.debug_vertical_scroll.content_length(n_lines);
        let scroll = state.debug_vertical_scroll.get_position();
        let paragraph = Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title("Debug")
                    .title_alignment(Alignment::Left)
                    .border_style(Style::default().fg(border_color))
                    .border_type(BorderType::Rounded),
            )
            .scroll((scroll as u16, 0))
            .alignment(HorizontalAlignment::Left)
            .style(panel_style);
        paragraph.render(area, buf);
        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            &mut state.debug_vertical_scroll,
        );
    }
}
