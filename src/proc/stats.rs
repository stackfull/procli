use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct ProcessStats {
    pub timestamp: Instant,
    pub cpu_percent: f32,
    pub memory_mb: f32,
    pub uptime: Duration,
}
impl ProcessStats {
    pub(crate) fn new(timestamp: Instant, info: &sysinfo::Process) -> Self {
        Self {
            timestamp,
            cpu_percent: info.cpu_usage(),
            memory_mb: info.memory() as f32 / 1_000_000.0,
            uptime: Duration::from_secs(info.run_time()),
        }
    }
}

impl Default for ProcessStats {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            cpu_percent: 0.0,
            memory_mb: 0.0,
            uptime: Duration::ZERO,
        }
    }
}
