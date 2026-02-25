use rat_widget::focus::{FocusFlag, HasFocus};
use ratatui::{macros::*, prelude::*, widgets::*};

use crate::{config::ProcliConfig, proc::process::Process, ui::theme::Theme};

#[derive(Debug)]
pub struct DebugWidget<'a> {
    pub theme: Theme,
    pub processes: &'a [Process],
    pub config: &'a ProcliConfig,
}

/// State for the [DebugWidget]
#[derive(Debug)]
pub struct DebugWidgetState {
    pub focus: FocusFlag,
    pub area: Rect,
    pub vertical_scroll: ScrollbarState,
}

impl Default for DebugWidgetState {
    fn default() -> Self {
        Self {
            focus: FocusFlag::new().with_name("debug"),
            area: Default::default(),
            vertical_scroll: ScrollbarState::new(1),
        }
    }
}

impl HasFocus for DebugWidgetState {
    fn build(&self, builder: &mut rat_widget::focus::FocusBuilder) {
        let tag = builder.start(self);
        builder.end(tag);
    }

    fn focus(&self) -> rat_widget::focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }
}

impl StatefulWidget for &DebugWidget<'_> {
    type State = DebugWidgetState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.area = area;
        let border_color = match state.focus.is_focused() {
            true => self.theme.accent,
            false => self.theme.foreground,
        };
        let panel_style = Style::default()
            .bg(self.theme.surface)
            .fg(self.theme.foreground);

        let debug = &self;
        let s = format!("{debug:#?}");
        let lines: Vec<Line> = s.lines().map(|sl| Line::from(sl)).collect();
        let n_lines = lines.len();
        state.vertical_scroll = state.vertical_scroll.content_length(n_lines);
        let scroll = state.vertical_scroll.get_position();
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
            &mut state.vertical_scroll,
        );
    }
}
