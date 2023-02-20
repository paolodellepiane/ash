pub use eyre::{bail, ensure, eyre, Context, ContextCompat, Result, WrapErr};
pub use std::format as f;
pub use std::println as p;
use std::time::Instant;

#[allow(dead_code)]
pub fn stopwatch_guard(name: &str) -> StopwatchGuard {
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

#[allow(unused_macros)]
macro_rules! stopwatch {
    () => {
        let ___stopwatch_guard = stopwatch_guard(&f!("fn at {}:{}", file!(), line!()));
    };
    ($e:expr) => {
        let ___stopwatch_guard = stopwatch_guard($e);
    };
}

#[allow(unused_imports)]
pub(crate) use stopwatch;

#[allow(dead_code)]
pub fn fst<F, S>(x: (F, S)) -> F {
    x.0
}

#[allow(dead_code)]
pub fn snd<F, S>(x: (F, S)) -> S {
    x.1
}

pub trait Inspect<T, E> {
    fn tap(self, f: impl FnOnce(&Result<T, E>)) -> Result<T, E>;
    fn tap_err(self, f: impl FnOnce(&E)) -> Result<T, E>;
}

impl<T, E> Inspect<T, E> for Result<T, E> {
    fn tap(self, f: impl FnOnce(&Result<T, E>)) -> Result<T, E> {
        f(&self);
        self
    }

    fn tap_err(self, f: impl FnOnce(&E)) -> Result<T, E> {
        match self {
            Ok(_) => self,
            Err(ref err) => {
                f(err);
                self
            }
        }
    }
}
