pub use anyhow::{anyhow, bail, ensure, Context, Result};
pub use std::format as f;
pub use std::println as p;
use std::time::Instant;

pub trait OptionStringExt<S>
where
    S: AsRef<str>,
{
    fn is_none_or_empty(&self) -> bool;
    fn not_empty(self) -> Option<S>;
}

impl<S> OptionStringExt<S> for Option<S>
where
    S: AsRef<str>,
{
    fn is_none_or_empty(&self) -> bool {
        self.is_none() || self.as_ref().unwrap().as_ref().is_empty()
    }

    fn not_empty(self) -> Option<S> {
        self.filter(|x| !x.as_ref().is_empty())
    }
}

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
