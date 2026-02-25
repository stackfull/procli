use std::fmt::Debug;

use crate::ui::{debug::DebugWidgetState, proc_card::ProcessGridState};
use rat_widget::focus::{FocusFlag, HasFocus, impl_has_focus};
use ratatui::{prelude::Rect, widgets::ScrollbarState};

#[derive(Debug)]
pub struct UiState {
    pub cards: ProcessGridState,
    pub logs: LogView,
    pub debug: DebugWidgetState,
}

#[derive(Debug)]
pub struct LogView {
    pub focus: FocusFlag,
    pub area: Rect,
    pub v_scroll: ScrollbarState,
    pub h_scroll: ScrollbarState,
}

impl Default for LogView {
    fn default() -> Self {
        Self {
            focus: FocusFlag::new().with_name("debug"),
            area: Default::default(),
            v_scroll: ScrollbarState::new(1),
            h_scroll: ScrollbarState::new(1),
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            logs: Default::default(),
            cards: Default::default(),
            debug: Default::default(),
        }
    }
}

impl_has_focus!(cards, logs, debug for UiState);

impl HasFocus for LogView {
    fn build(&self, builder: &mut rat_widget::focus::FocusBuilder) {
        let tag = builder.start(self);
        // builder.widget(self.h_scroll);
        // builder.widget(self.v_scroll);
        builder.end(tag);
    }

    fn focus(&self) -> rat_widget::focus::FocusFlag {
        self.focus.clone()
    }

    fn area(&self) -> ratatui::layout::Rect {
        self.area
    }
}

const TICK_FPS: f64 = 30.0;

impl UiState {
    // pub fn tick(&mut self) {
    //     self.tick += 1.0;
    //     if self.tick > 2.0 * TICK_FPS {
    //         self.tick = 0.0;
    //         self.time = Instant::now();
    //     }
    // }

    // pub fn step_of_8_in_1_second(&self) -> usize {
    //     (self.tick * 8.0 / TICK_FPS) as usize % 8
    // }

    // pub fn step_of_4_in_1_second(&self) -> usize {
    //     (self.tick * 4.0 / TICK_FPS) as usize % 4
    // }

    // pub fn step_of_8_in_2_second(&self) -> usize {
    //     (self.tick * 4.0 / TICK_FPS) as usize % 8
    // }

    // pub fn focus_next(&mut self) {
    //     self.focus = match &self.focus {
    //         None => Some(Focussable::Process(0)),
    //         Some(Focussable::Process(i)) => {
    //             if i + 1 < self.procs {
    //                 Some(Focussable::Process(i + 1))
    //             } else {
    //                 Some(Focussable::Logs)
    //             }
    //         }
    //         Some(Focussable::Logs) => {
    //             if self.debug {
    //                 Some(Focussable::Debug)
    //             } else {
    //                 Some(Focussable::Process(0))
    //             }
    //         }
    //         Some(Focussable::Debug) => Some(Focussable::Process(0)),
    //     }
    // }

    // pub fn focus_prev(&mut self) {
    //     self.focus = match &self.focus {
    //         None => Some(Focussable::Process(0)),
    //         Some(Focussable::Process(i)) => {
    //             if *i > 0 {
    //                 Some(Focussable::Process(i - 1))
    //             } else if self.debug {
    //                 Some(Focussable::Debug)
    //             } else {
    //                 Some(Focussable::Logs)
    //             }
    //         }
    //         Some(Focussable::Logs) => {
    //             if self.procs > 0 {
    //                 Some(Focussable::Process(self.procs - 1))
    //             } else if self.debug {
    //                 Some(Focussable::Debug)
    //             } else {
    //                 Some(Focussable::Logs)
    //             }
    //         }
    //         Some(Focussable::Debug) => Some(Focussable::Logs),
    //     }
    // }

    // pub fn update_procs(&mut self, count: usize) {
    //     self.procs = count;
    //     if let Some(Focussable::Process(idx)) = &self.focus
    //         && *idx >= self.procs
    //     {
    //         self.focus = Some(if self.procs == 0 {
    //             Focussable::Logs
    //         } else {
    //             Focussable::Process(self.procs - 1)
    //         });
    //     }
    // }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     const TICKS_AND_STEPS: [(usize, usize, usize, usize); 13] = [
//         (0, 0, 0, 0),
//         (1, 0, 0, 0),
//         (2, 0, 0, 0),
//         (1, 0, 1, 0),
//         (3, 0, 1, 0),
//         (1, 1, 2, 1),
//         (3, 1, 2, 1),
//         (1, 1, 3, 1),
//         (2, 1, 3, 1),
//         (1, 2, 4, 2),
//         (15, 0, 0, 4),
//         (15, 2, 4, 6),
//         (15, 0, 0, 0),
//     ];

//     #[test]
//     fn all_the_throbs() {
//         let mut t = UiState::default();
//         let mut c = 0;
//         for (ticks, s4i1, s8i1, s8i2) in TICKS_AND_STEPS {
//             for _ in 0..ticks {
//                 t.tick();
//                 c += 1;
//             }
//             assert_eq!(
//                 t.step_of_4_in_1_second(),
//                 s4i1,
//                 "After {} ticks, 4/1 should be {}",
//                 c,
//                 s4i1
//             );
//             assert_eq!(
//                 t.step_of_8_in_1_second(),
//                 s8i1,
//                 "After {} ticks, 8/1 should be {}",
//                 c,
//                 s8i1
//             );
//             assert_eq!(
//                 t.step_of_8_in_2_second(),
//                 s8i2,
//                 "After {} ticks, 8/2 should be {}",
//                 c,
//                 s8i2
//             );
//         }
//     }
// }
