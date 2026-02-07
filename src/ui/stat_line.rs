use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{proc::stats::ProcessStats, ui::state::UiState};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    macros::*,
    prelude::*,
    style::Stylize,
    widgets::*,
};

#[derive(Debug)]
pub struct SingleStat<'a> {
    name: String,
    unit: String,
    history: Vec<f32>,
    max: f32,
    timestamps: Vec<Instant>,
    ui: &'a UiState,
}

impl<'a> SingleStat<'a> {
    pub fn data(&self) -> Vec<(f64, f64)> {
        let now = Instant::now();
        std::iter::zip(&self.timestamps, &self.history)
            .map(|(x, y)| (-now.duration_since(*x).as_secs_f64(), *y as f64))
            .collect()
    }
}

pub fn split_stats<'a>(
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
