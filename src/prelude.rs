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

macro_rules! stopwatch {
    () => {
        let ___stopwatch_guard = stopwatch_guard(&f!("fn at {}:{}", file!(), line!()));
    };
    ($e:expr) => {
        let ___stopwatch_guard = stopwatch_guard($e);
    };
}

pub(crate) use stopwatch;

#[allow(dead_code)]
pub fn fst<F, S>(x: (F, S)) -> F {
    x.0
}

#[allow(dead_code)]
pub fn snd<F, S>(x: (F, S)) -> S {
    x.1
}

pub trait FltkWidgetExt {
    fn state(&mut self, activate: bool);
}

impl<T> FltkWidgetExt for T
where
    T: fltk::prelude::WidgetExt,
{
    fn state(&mut self, activate: bool) {
        if activate {
            self.activate();
        } else {
            self.deactivate();
        }
    }
}

pub mod fltkext {
    pub mod app {
        use crate::prelude::FltkColorExt;

        pub fn set_colors(
            background: fltk::enums::Color,
            background2: fltk::enums::Color,
            foreground: fltk::enums::Color,
        ) {
            background.apply_to_app_background();
            background2.apply_to_app_background2();
            foreground.apply_to_app_foreground();
        }
    }
}

pub trait FltkColorExt {
    fn apply_to_app_background(&self);
    fn apply_to_app_background2(&self);
    fn apply_to_app_foreground(&self);
}

impl FltkColorExt for fltk::enums::Color {
    fn apply_to_app_background(&self) {
        let (r, g, b) = self.to_rgb();
        fltk::app::background(r, g, b);
    }
    fn apply_to_app_background2(&self) {
        let (r, g, b) = self.to_rgb();
        fltk::app::background2(r, g, b);
    }
    fn apply_to_app_foreground(&self) {
        let (r, g, b) = self.to_rgb();
        fltk::app::foreground(r, g, b);
    }
}
