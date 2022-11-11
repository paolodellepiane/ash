pub use eyre::{bail, ensure, eyre, Context, ContextCompat, Result, WrapErr};
pub use std::format as f;
pub use std::println as p;
use std::time::Instant;

#[allow(dead_code)]
pub fn stopwatch(name: &str) -> StopwatchGuard {
    let start = Instant::now();
    StopwatchGuard { name: name.to_string(), start }
}

pub struct StopwatchGuard {
    name: String,
    start: Instant,
}

impl Drop for StopwatchGuard {
    fn drop(&mut self) {
        p!("{} took {}ms", self.name, self.start.elapsed().as_millis())
    }
}
