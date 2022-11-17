#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all)]
use aws::update_sshconfig;
use config::{Config, CFG};
use executable::*;
use fltk::app::Sender;
use fltk::enums::{Align, Color, Event, FrameType};
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

fn main() -> Result<()> {
    let config = &*CFG;
    if config.update {
        update_sshconfig(
            &config.keys_path,
            &Config::template_path(),
            &config.bastion_name,
        )?;
    }
    let matcher = SkimMatcherV2::default().ignore_case();

    let app = app::App::default().load_system_fonts();
    fltkext::app::set_colors(*windowBackgroundColor, *controlAccentColor, *labelColor);
    app::set_color(Color::Selection, 255, 255, 255);
    let widget_scheme = WidgetScheme::new(SchemeType::Aqua);
    widget_scheme.apply();
    let (s, r) = app::channel();

    let mut win = window::Window::default().with_size(400, 300);

    let mut main = group::Flex::default().size_of_parent().column();
    main.set_margin(10);
    main.set_pad(10);
    let mut filters = group::Flex::default().row();
    main.set_size(&filters, 30);
    let mut filter = input::Input::default();
    filter.set_color(*controlColor);
    filter.set_trigger(enums::CallbackTrigger::Changed);
    filter.emit(s.clone(), Msg::FilterChanged);
    let mut profile = menu_choice("");
    filters.set_size(&profile, 70);
    profile.emit(s.clone(), Msg::FilterChanged);
    let mut platform = menu_choice("");
    filters.set_size(&platform, 70);
    platform.emit(s.clone(), Msg::FilterChanged);
    let mut update = button::Button::default().with_label("\u{f0de}").with_size(20, 20);
    update.set_label_font(enums::Font::by_name("Wingdings-Regular"));
    update.set_label_size(20);
    filters.set_size(&update, 20);
    update.set_color(Color::BackGround);
    update.set_selection_color(Color::BackGround);
    update.set_label_color(*systemGrayColor);
    update.set_frame(FrameType::OFlatFrame);
    update.emit(s.clone(), Msg::UpdateSsh);
    filters.end();

    let mut br = browser::HoldBrowser::default();
    br.style();
    br.set_column_widths(&[300, 80]);
    br.set_column_char('\t');
    br.handle({
        let s = s.clone();
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
    ssh.emit(s.clone(), Msg::Exec(Cmd::Ssh));
    let mut commands = menu_button(&Cmd::iter().map(|x| x.as_ref().to_lowercase()).join("|"));
    buttons.set_size(&commands, 30);
    commands.set_callback({
        let s = s.clone();
        move |x| s.send(Msg::Exec(Cmd::from_repr(x.value() as usize).unwrap()))
    });
    buttons.end();

    main.end();
    win.end();
    win.make_resizable(true);
    win.show();

    let mut update = |dispatch: Sender<Msg>, state: &mut ExecOpt, msg: Msg| -> Result<()> {
        match msg {
            Msg::Exec(cmd) if br.selected_text().is_some() => {
                let sel = br.selected_text().unwrap();
                let host = sel
                    .split_once('\t')
                    .map(fst)
                    .ok_or_else(|| eyre!("Selected text is empty"))?
                    .to_string();
                match cmd {
                    Cmd::Ssh => Ssh::new(&ExecOpt { host, ..state.clone() })?.exec()?,
                    Cmd::Rdp => Tunnel::new("rdp", &ExecOpt { host, ..state.clone() })?.exec()?,
                    Cmd::Code => Code::new(&ExecOpt { host, ..state.clone() })?.exec()?,
                }
            }
            Msg::Exec(_) => (),
            Msg::HostChanged => {
                ssh.state(br.selected_text().is_some());
                commands.state(br.selected_text().is_some());
            }
            Msg::FilterChanged => {
                br.clear();
                for (_, k, v) in state
                    .hosts
                    .iter()
                    .sorted_by_key(|&(k, _)| k)
                    .filter(|(_, h)| {
                        if profile.value() == 0 {
                            true
                        } else {
                            h.profile == profile.text(profile.value()).expect("invalid profile")
                        }
                    })
                    .filter(|(_, h)| {
                        if platform.value() == 0 {
                            true
                        } else {
                            h.platform == platform.text(platform.value()).expect("invalid platform")
                        }
                    })
                    .filter_map(|(k, h @ Host { profile, .. })| {
                        matcher
                            .fuzzy_match(&f!("{profile}:{k}:{profile}"), filter.value().trim())
                            .map(|score| (score, k, h))
                    })
                    .sorted_by_key(|&(score, _, _)| Reverse(score))
                {
                    br.add(&f!("{k}\t{}", v.profile));
                }
                if br.size() > 0 {
                    br.select(1);
                }
                ssh.state(br.selected_text().is_some());
                commands.state(br.selected_text().is_some());
            }
            Msg::UpdateSsh => {
                update_sshconfig(
                    &config.keys_path,
                    &Config::template_path(),
                    &config.bastion_name,
                )?;
                dispatch.send(Msg::Load)
            }
            Msg::Load => {
                let hosts = parse_ssh_config_from_host()?;
                state.hosts = hosts;
                let profiles = ["all"]
                    .into_iter()
                    .chain(state.hosts.iter().map(|(_, h)| h.profile.as_str()).sorted().unique())
                    .join("|");
                profile.clear();
                profile.add_choice(&profiles);
                profile.set_value(0);
                let platforms = ["all"]
                    .into_iter()
                    .chain(state.hosts.iter().map(|(_, h)| h.platform.as_str()).sorted().unique())
                    .join("|");
                platform.clear();
                platform.add_choice(&platforms);
                platform.set_value(0);
                dispatch.send(Msg::FilterChanged);
            }
        }
        Ok(())
    };

    let mut state = ExecOpt::default();
    s.send(Msg::Load);
    while app.wait() {
        if let Some(msg) = r.recv() {
            match update(s.clone(), &mut state, msg) {
                Ok(()) => {}
                Err(err) => p!("{err:?}"),
            }
        }
    }
    Ok(())
}
