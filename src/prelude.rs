pub use crate::option_not_empty_string::*;
pub use anyhow::{anyhow, bail, ensure, Context, Result};
pub use std::format as f;
use std::ops::Deref;
pub use std::println as p;
use std::time::Instant;

pub trait ThrutyOptionStringExt {
    fn is_falsy(&self) -> bool;
    fn is_truthy(&self) -> bool;
}

impl<S> ThrutyOptionStringExt for Option<S> where S: Deref<Target = str> {
    fn is_falsy(&self) -> bool {
        self.is_none() || self.as_ref().unwrap().is_empty()
    }

    fn is_truthy(&self) -> bool {
        !self.is_falsy()
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
