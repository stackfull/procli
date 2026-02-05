use std::{fmt::Debug, time::Instant};

use crate::{event::TICK_FPS, ui::theme::Theme};
use tui_logger::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Focussable {
    Process(usize),
    Logs,
    Debug,
}

/// The main UI mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// All processes and logs
    Dashboard,
    /// Spotlight a single process
    Spotlight,
    /// Large log split view
    Logs,
}

pub struct UiState {
    pub tick: f64,
    pub time: Instant,
    pub proc_columns: usize,
    pub proc_rows: usize,
    pub theme: Theme,
    pub procs: usize,
    pub focus: Option<Focussable>,
    pub mode: Mode,
    pub debug: bool,
    pub logger_state: TuiWidgetState,
}

impl Debug for UiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiState")
            .field("tick", &self.tick)
            .field("time", &self.time)
            .field("proc_columns", &self.proc_columns)
            .field("proc_rows", &self.proc_rows)
            .field("procs", &self.procs)
            .field("mode", &self.mode)
            .field("focus", &self.focus)
            .finish()
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            logger_state: TuiWidgetState::new(),
            tick: Default::default(),
            time: Instant::now(),
            proc_columns: 2,
            proc_rows: 3,
            procs: 0,
            theme: Theme::dark(),
            mode: Mode::Dashboard,
            focus: None,
            debug: false,
        }
    }
}

impl UiState {
    pub fn tick(&mut self) {
        self.tick += 1.0;
        if self.tick > 2.0 * TICK_FPS {
            self.tick = 0.0;
            self.time = Instant::now();
        }
    }

    pub fn step_of_8_in_1_second(&self) -> usize {
        (self.tick * 8.0 / TICK_FPS) as usize % 8
    }

    pub fn step_of_4_in_1_second(&self) -> usize {
        (self.tick * 4.0 / TICK_FPS) as usize % 4
    }

    pub fn step_of_8_in_2_second(&self) -> usize {
        (self.tick * 4.0 / TICK_FPS) as usize % 8
    }

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
        if !self.debug
            && let Some(Focussable::Debug) = &self.focus
        {
            self.focus = Some(Focussable::Process(0));
        }
    }

    pub fn focus_next(&mut self) {
        self.focus = match &self.focus {
            None => Some(Focussable::Process(0)),
            Some(Focussable::Process(i)) => {
                if i + 1 < self.procs {
                    Some(Focussable::Process(i + 1))
                } else {
                    Some(Focussable::Logs)
                }
            }
            Some(Focussable::Logs) => {
                if self.debug {
                    Some(Focussable::Debug)
                } else {
                    Some(Focussable::Process(0))
                }
            }
            Some(Focussable::Debug) => Some(Focussable::Process(0)),
        }
    }

    pub fn focus_prev(&mut self) {
        self.focus = match &self.focus {
            None => Some(Focussable::Process(0)),
            Some(Focussable::Process(i)) => {
                if *i > 0 {
                    Some(Focussable::Process(i - 1))
                } else if self.debug {
                    Some(Focussable::Debug)
                } else {
                    Some(Focussable::Logs)
                }
            }
            Some(Focussable::Logs) => {
                if self.procs > 0 {
                    Some(Focussable::Process(self.procs - 1))
                } else if self.debug {
                    Some(Focussable::Debug)
                } else {
                    Some(Focussable::Logs)
                }
            }
            Some(Focussable::Debug) => Some(Focussable::Logs),
        }
    }

    pub fn update_procs(&mut self, count: usize) {
        self.procs = count;
        if let Some(Focussable::Process(idx)) = &self.focus
            && *idx >= self.procs
        {
            self.focus = Some(if self.procs == 0 {
                Focussable::Logs
            } else {
                Focussable::Process(self.procs - 1)
            });
        }
    }

    pub fn toggle_spotlight(&mut self) {
        if self.mode == Mode::Spotlight {
            self.mode = Mode::Dashboard;
        } else {
            self.mode = Mode::Spotlight;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICKS_AND_STEPS: [(usize, usize, usize, usize); 13] = [
        (0, 0, 0, 0),
        (1, 0, 0, 0),
        (2, 0, 0, 0),
        (1, 0, 1, 0),
        (3, 0, 1, 0),
        (1, 1, 2, 1),
        (3, 1, 2, 1),
        (1, 1, 3, 1),
        (2, 1, 3, 1),
        (1, 2, 4, 2),
        (15, 0, 0, 4),
        (15, 2, 4, 6),
        (15, 0, 0, 0),
    ];

    #[test]
    fn all_the_throbs() {
        let mut t = UiState::default();
        let mut c = 0;
        for (ticks, s4i1, s8i1, s8i2) in TICKS_AND_STEPS {
            for _ in 0..ticks {
                t.tick();
                c += 1;
            }
            assert_eq!(
                t.step_of_4_in_1_second(),
                s4i1,
                "After {} ticks, 4/1 should be {}",
                c,
                s4i1
            );
            assert_eq!(
                t.step_of_8_in_1_second(),
                s8i1,
                "After {} ticks, 8/1 should be {}",
                c,
                s8i1
            );
            assert_eq!(
                t.step_of_8_in_2_second(),
                s8i2,
                "After {} ticks, 8/2 should be {}",
                c,
                s8i2
            );
        }
    }
}
