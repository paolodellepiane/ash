use dioxus::prelude::Component;
pub use eyre::{bail, ensure, eyre, Context, ContextCompat, Result, WrapErr};
use std::cell::RefCell;
pub use std::format as f;
pub use std::println as p;
use std::rc::Rc;
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

pub struct WithResult<Props: 'static, Response: 'static> {
    pub props: Props,
    pub result: Rc<RefCell<Option<Response>>>,
}

pub fn launch<Props: 'static, Response: 'static>(
    app: Component<WithResult<Props, Response>>,
    props: Props,
) -> Option<Response> {
    let result = Rc::new(RefCell::new(None));
    let wr = WithResult { props, result: result.clone() };
    dioxus_tui::launch_cfg_with_props(app, wr, dioxus_tui::Config::new());
    let mut res = result.borrow_mut();
    res.take()
}
