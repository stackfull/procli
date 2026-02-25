use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{proc::stats::ProcessStats, ui::theme::Theme};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    macros::*,
    prelude::*,
    style::Stylize,
    widgets::*,
};

#[derive(Debug)]
pub struct SingleStat {
    name: String,
    unit: String,
    history: Vec<f32>,
    max: f32,
    timestamps: Vec<Instant>,
    time: Instant,
    theme: Theme,
}

impl SingleStat {
    pub fn data(&self) -> Vec<(f64, f64)> {
        std::iter::zip(&self.timestamps, &self.history)
            .map(|(x, y)| (-self.time.duration_since(*x).as_secs_f64(), *y as f64))
            .collect()
    }
}

pub fn split_stats<'a>(
    stats: &[ProcessStats],
    max_stats: &ProcessStats,
    now: Instant,
    theme: Theme,
) -> (SingleStat, SingleStat) {
    let timestamps: Vec<Instant> = stats.iter().map(|s| s.timestamp).collect();
    let cpu_history = SingleStat {
        name: "CPU".to_string(),
        unit: "%".to_string(),
        history: stats.iter().map(|s| s.cpu_percent).collect(),
        max: max_stats.cpu_percent,
        timestamps: timestamps.clone(),
        time: now,
        theme,
    };
    let mem_history = SingleStat {
        name: "RAM".to_string(),
        unit: "MB".to_string(),
        history: stats.iter().map(|s| s.memory_mb).collect(),
        max: max_stats.memory_mb,
        timestamps,
        time: now,
        theme,
    };
    (cpu_history, mem_history)
}

impl Widget for &SingleStat {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [_, history, _, label, current, _] =
            horizontal![==1, *=1, ==1, ==6, ==8, ==2].areas(area);
        Text::from(self.name.clone() + ":").render(label, buf);
        ratatui::macros::line![
            span![format!("{:.1}", self.history.last().unwrap_or(&0.0))],
            span![format!("{:<2}", self.unit.clone())].fg(self.theme.primary_background)
        ]
        .alignment(Alignment::Right)
        .render(current, buf);
        let resampled: Vec<Option<u64>> = crate::resample::resample(
            &self.history,
            &self.timestamps,
            self.time - Duration::from_secs(120),
            self.time,
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
            .fg(self.theme.primary)
            .render(history, buf);
    }
}
