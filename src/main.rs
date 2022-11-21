#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all)]
use aws::update_sshconfig;
use config::Config;
use executable::*;
use fltk::app::{Receiver, Sender};
use fltk::enums::{Align, Color, Event, FrameType};
use fltk::menu::MenuButton;
use fltk::{prelude::*, *};
use fltk_theme::colors::aqua::dark::*;
use fltk_theme::widget_schemes::aqua::frames::*;
use fltk_theme::{SchemeType, WidgetScheme};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use parsers::ssh_config_parser::{parse_ssh_config_from_host, Host};
use prelude::*;
use std::cmp::Reverse;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter, FromRepr};

mod aws;
mod config;
mod describe_instances;
mod executable;
mod parsers;
mod prelude;

#[derive(Clone, EnumIter, AsRefStr, FromRepr)]
enum Cmd {
    Ssh,
    Rdp,
    Code,
}

#[derive(Clone)]
enum Msg {
    Load,
    Exec(Cmd),
    HostChanged,
    FilterChanged,
    UpdateSsh,
}

trait Pippo {
    fn style(&mut self);
}

impl<T: WidgetExt> Pippo for T {
    fn style(&mut self) {
        self.set_color(*controlColor);
        self.set_selection_color(*controlAccentColor);
        self.set_frame(OS_DEFAULT_BUTTON_UP_BOX);
    }
}

fn button(label: &str) -> button::Button {
    let mut w = button::Button::default().with_label(label);
    w.style();
    w
}

fn menu_button(choices: &str) -> menu::MenuButton {
    let mut w = menu::MenuButton::default();
    w.style();
    w.set_label_font(enums::Font::by_name("Wingdings-Regular"));
    w.set_label_size(20);
    w.add_choice(choices);
    w.draw(|b| {
        draw::set_draw_color(Color::BackGround);
        draw::draw_rectf(b.x(), b.y(), b.width(), b.height());
        draw::set_draw_color(*systemGrayColor);
        draw::set_font(enums::Font::by_name("AppleSymbols"), b.height());
        draw::draw_text2(
            "\u{22ee}",
            b.x(),
            b.y() + 5,
            b.width(),
            b.height(),
            Align::Center,
        );
    });
    w
}

fn menu_choice(choices: &str) -> menu::Choice {
    let mut w = menu::Choice::default();
    w.style();
    w.add_choice(choices);
    w.set_value(0);
    w
}

#[derive(Clone)]
struct App {
    cfg: Config,
    app: app::App,
    browser: browser::HoldBrowser,
    ssh: button::Button,
    commands: MenuButton,
    filter: input::Input,
    profile: menu::Choice,
    platform: menu::Choice,
    sender: Sender<Msg>,
    receiver: Receiver<Msg>,
    state: ExecOpt,
}

impl App {
    #[rustfmt::skip]
    fn new(cfg: Config) -> Self {
        let app = app::App::default().load_system_fonts();
        fltkext::app::set_colors(*windowBackgroundColor, *controlAccentColor, *labelColor);
        app::set_color(Color::Selection, 255, 255, 255);
        let widget_scheme = WidgetScheme::new(SchemeType::Aqua);
        widget_scheme.apply();
        let (sender, receiver) = app::channel();

        let mut win = window::Window::default().with_size(400, 300);
        win.make_resizable(true);
        let mut main = group::Flex::default().size_of_parent().column();
            main.set_margin(10);
            main.set_pad(10);

            let mut filters = group::Flex::default().row();
                main.set_size(&filters, 30);
                let mut filter = input::Input::default();
                    filter.set_color(*controlColor);
                    filter.set_trigger(enums::CallbackTrigger::Changed);
                    filter.emit(sender.clone(), Msg::FilterChanged);
                let mut profile = menu_choice("");
                    profile.emit(sender.clone(), Msg::FilterChanged);
                let mut platform = menu_choice("");
                    platform.emit(sender.clone(), Msg::FilterChanged);
                let mut update = button::Button::default().with_label("\u{f0de}").with_size(20, 20);
                    update.set_label_font(enums::Font::by_name("Wingdings-Regular"));
                    update.set_label_size(20);
                    update.set_color(Color::BackGround);
                    update.set_selection_color(Color::BackGround);
                    update.set_label_color(*systemGrayColor);
                    update.set_frame(FrameType::OFlatFrame);
                    update.emit(sender.clone(), Msg::UpdateSsh);
                filters.set_size(&profile, 70);
                filters.set_size(&platform, 70);
                filters.set_size(&update, 20);
            filters.end();

            let mut browser = browser::HoldBrowser::default();
                browser.style();
                browser.set_column_widths(&[300, 80]);
                browser.set_column_char('\t');
                browser.handle({
                    let s = sender.clone();
                    move |_, event| match event {
                        Event::Released if app::event_clicks() => {
                            s.send(Msg::Exec(Cmd::Ssh));
                            true
                        }
                        _ => {
                            s.send(Msg::HostChanged);
                            false
                        }
                    }
                });

            let mut buttons = group::Flex::default().row();
                main.set_size(&buttons, 30);
                let mut ssh = button("ssh");
                ssh.emit(sender.clone(), Msg::Exec(Cmd::Ssh));
                let mut commands = menu_button(&Cmd::iter().map(|x| x.as_ref().to_lowercase()).join("|"));
                commands.set_callback({
                    let s = sender.clone();
                    move |x| s.send(Msg::Exec(Cmd::from_repr(x.value() as usize).unwrap()))
                });
                buttons.set_size(&commands, 30);
            buttons.end();

        main.end();
        win.end();
        win.show();

        let hosts = parse_ssh_config_from_host().unwrap_or_default();
        let state = ExecOpt { hosts, ..ExecOpt::default() };
        App { cfg, app, browser, ssh, commands, filter, profile, platform, state, sender, receiver, }
    }

    fn run(mut self) -> Result<()> {
        self.sender.send(Msg::Load);
        while self.app.wait() {
            if let Some(msg) = self.receiver.recv() {
                match self.update(msg) {
                    Ok(()) => {}
                    Err(err) => p!("{err:?}"),
                }
            }
        }
        Ok(())
    }

    fn update(&mut self, msg: Msg) -> Result<()> {
        let matcher = SkimMatcherV2::default().ignore_case();
        match msg {
            Msg::Exec(cmd) if self.browser.selected_text().is_some() => {
                let sel = self.browser.selected_text().unwrap();
                let host = sel
                    .split_once('\t')
                    .map(fst)
                    .ok_or_else(|| eyre!("Selected text is empty"))?
                    .to_string();
                match cmd {
                    Cmd::Ssh => Ssh::new(&ExecOpt { host, ..self.state.clone() })?.exec()?,
                    Cmd::Rdp => {
                        Tunnel::new("rdp", &ExecOpt { host, ..self.state.clone() }, &self.cfg)?
                            .exec()?
                    }
                    Cmd::Code => Code::new(&ExecOpt { host, ..self.state.clone() })?.exec()?,
                }
            }
            Msg::Exec(_) => (),
            Msg::HostChanged => {
                self.ssh.state(self.browser.selected_text().is_some());
                self.commands.state(self.browser.selected_text().is_some());
            }
            Msg::FilterChanged => {
                self.browser.clear();
                for (_, k, v) in self
                    .state
                    .hosts
                    .iter()
                    .sorted_by_key(|&(k, _)| k)
                    .filter(|(_, h)| {
                        if self.profile.value() == 0 {
                            true
                        } else {
                            h.profile
                                == self.profile.text(self.profile.value()).expect("invalid profile")
                        }
                    })
                    .filter(|(_, h)| {
                        if self.platform.value() == 0 {
                            true
                        } else {
                            h.platform
                                == self
                                    .platform
                                    .text(self.platform.value())
                                    .expect("invalid platform")
                        }
                    })
                    .filter_map(|(k, h @ Host { profile, .. })| {
                        matcher
                            .fuzzy_match(&f!("{profile}:{k}:{profile}"), self.filter.value().trim())
                            .map(|score| (score, k, h))
                    })
                    .sorted_by_key(|&(score, _, _)| Reverse(score))
                {
                    self.browser.add(&f!("{k}\t{}", v.profile));
                }
                if self.browser.size() > 0 {
                    self.browser.select(1);
                }
                self.ssh.state(self.browser.selected_text().is_some());
                self.commands.state(self.browser.selected_text().is_some());
            }
            Msg::UpdateSsh => {
                update_sshconfig(
                    &self.cfg.keys_path,
                    &Config::template_path(),
                    &self.cfg.bastion_name,
                )?;
                self.sender.send(Msg::Load)
            }
            Msg::Load => {
                let hosts = parse_ssh_config_from_host().unwrap_or_default();
                self.state.hosts = hosts;
                let profiles = ["all"]
                    .into_iter()
                    .chain(
                        self.state.hosts.iter().map(|(_, h)| h.profile.as_str()).sorted().unique(),
                    )
                    .join("|");
                self.profile.clear();
                self.profile.add_choice(&profiles);
                self.profile.set_value(0);
                let platforms = ["all"]
                    .into_iter()
                    .chain(
                        self.state.hosts.iter().map(|(_, h)| h.platform.as_str()).sorted().unique(),
                    )
                    .join("|");
                self.platform.clear();
                self.platform.add_choice(&platforms);
                self.platform.set_value(0);
                self.sender.send(Msg::FilterChanged);
            }
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let cfg = Config::load().context("Can't load config")?;
    let app = App::new(cfg);
    app.run()
}
