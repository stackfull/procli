use rat_salsa::event::RenderedEvent;
use ratatui::crossterm::event::Event as CrosstermEvent;
use std::process::ExitStatus;
use uuid::Uuid;

/// Representation of all possible events.
#[derive(Clone, Debug)]
pub enum AppEvent {
    /// Use this event to run any code which has to run outside of being a direct response to a user
    /// event. e.g. polling exernal systems, updating animations, or rendering the UI based on a
    /// fixed frame rate.
    Tick,
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Config file changed.
    Reload,
    Rendered,
    StatsRefresh,
    ProcessDied(Uuid, ExitStatus),
    ToggleDebug,
    ToggleSpotlight,
    /// Quit the application.
    Quit,
}

impl From<CrosstermEvent> for AppEvent {
    fn from(value: CrosstermEvent) -> Self {
        Self::Crossterm(value)
    }
}

impl From<RenderedEvent> for AppEvent {
    fn from(_: RenderedEvent) -> Self {
        Self::Rendered
    }
}
