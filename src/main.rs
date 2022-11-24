#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all)]
use aws::update_sshconfig;
use config::Config;
use eframe::epaint::ahash::HashMap;
use egui::{color::*, *};
use executable::*;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use parsers::ssh_config_parser::{parse_ssh_config_from_host, Host};
use prelude::*;
use std::{cmp::Reverse, str::FromStr};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter, EnumString};

mod aws;
mod config;
mod describe_instances;
mod executable;
mod parsers;
mod prelude;

#[derive(Clone, EnumIter, AsRefStr, EnumString)]
enum Cmd {
    Ssh,
    Rdp,
    Code,
}

struct MyApp {
    cfg: Config,
    state: ExecOpt,
    filter: String,
    profile: String,
    platform: String,
}

impl MyApp {
    fn filtered_hosts(&self) -> Vec<(i64, String, Host)> {
        let matcher = SkimMatcherV2::default().ignore_case();
        self.state
            .hosts
            .iter()
            .sorted_by_key(|&(k, _)| k)
            .filter(|(_, h)| self.profile == "all" || h.profile == self.profile)
            .filter(|(_, h)| self.platform == "all" || h.platform == self.platform)
            .filter_map(|(k, h @ Host { profile, .. })| {
                matcher
                    .fuzzy_match(&f!("{profile}:{k}:{profile}"), self.filter.trim())
                    .map(|score| (score, k, h))
            })
            .sorted_by_key(|&(score, _, _)| Reverse(score))
            .map(|(s, k, v)| (s, k.clone(), v.clone()))
            .collect_vec()
    }

    fn profiles(&self) -> Vec<String> {
        ["all"]
            .into_iter()
            .chain(self.state.hosts.iter().map(|(_, h)| h.profile.as_str()).sorted().unique())
            .map(String::from)
            .collect_vec()
    }

    fn platform(&self) -> Vec<String> {
        ["all"]
            .into_iter()
            .chain(self.state.hosts.iter().map(|(_, h)| h.platform.as_str()).sorted().unique())
            .map(String::from)
            .collect_vec()
    }
}

impl MyApp {
    fn new() -> Result<Self> {
        let hosts = match parse_ssh_config_from_host() {
            Ok(hosts) => hosts,
            Err(err) => {
                p!("{err:?}");
                Default::default()
            }
        };
        let state = ExecOpt { hosts, ..ExecOpt::default() };
        Ok(Self {
            cfg: Config::load().expect("can't load config"),
            state,
            filter: Default::default(),
            platform: "all".into(),
            profile: "all".into(),
        })
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                TextEdit::singleline(&mut self.filter).hint_text("search").show(ui);
                ComboBox::from_label("profile").selected_text(&self.profile).show_ui(ui, |ui| {
                    for p in self.profiles() {
                        ui.selectable_value(&mut self.profile, p.clone(), p);
                    }
                });
                ComboBox::from_label("platform").selected_text(&self.platform).show_ui(ui, |ui| {
                    for p in self.platform() {
                        ui.selectable_value(&mut self.platform, p.clone(), p);
                    }
                });
                if ui.button("r").clicked() {
                    match update_sshconfig(
                        &self.cfg.keys_path,
                        &Config::template_path(),
                        &self.cfg.bastion_name,
                    ) {
                        Ok(()) => {
                            self.state.hosts = parse_ssh_config_from_host().unwrap_or_default()
                        }
                        Err(err) => p!("{err:?}"),
                    }
                }
            });
            ScrollArea::vertical().max_height(200.0).auto_shrink([false; 2]).show(ui, |ui| {
                ui.vertical(|ui| {
                    for (_, k, v) in self.filtered_hosts() {
                        if ui
                            .selectable_value(
                                &mut self.state.host,
                                k.to_string(),
                                format!("{k} {}", v.profile),
                            )
                            .double_clicked()
                        {
                            p!("AAAAAAAA");
                        }
                    }
                });
            });
            ui.add_space(10.);
            ui.horizontal(|ui| {
                if ui.button("ssh").clicked() {}
                ui.menu_button("", |ui| {
                    for c in Cmd::iter() {
                        if ui.button(c.as_ref()).clicked() {
                            match Cmd::from_str(c.as_ref()).unwrap() {
                                Cmd::Ssh => Ssh::new(&self.state).unwrap().exec().unwrap(),
                                _ => (), // Cmd::Rdp => Tunnel::new("rdp", &self.state & self.cfg)?.exec()?,
                                         // Cmd::Code => {
                                         //     Code::new(&ExecOpt { host, ..self.state.clone() })?.exec()?
                                         // }
                            };
                            ui.close_menu();
                        }
                    }
                });
            })
        });
    }
}

// impl App {
//     #[rustfmt::skip]
//     fn new(cfg: Config) -> Self {
//         let app = app::App::default().load_system_fonts();
//         fltkext::app::set_colors(*windowBackgroundColor, *controlAccentColor, *labelColor);
//         app::set_color(Color::Selection, 255, 255, 255);
//         let widget_scheme = WidgetScheme::new(SchemeType::Aqua);
//         widget_scheme.apply();
//         let (sender, receiver) = app::channel();

//         let mut win = window::Window::default().with_size(400, 300);
//         win.make_resizable(true);
//         let mut main = group::Flex::default().size_of_parent().column();
//             main.set_margin(10);
//             main.set_pad(10);

//             let mut filters = group::Flex::default().row();
//                 main.set_size(&filters, 30);
//                 let mut filter = input::Input::default();
//                     filter.set_color(*controlColor);
//                     filter.set_trigger(enums::CallbackTrigger::Changed);
//                     filter.emit(sender.clone(), Msg::FilterChanged);
//                 let mut profile = menu_choice("");
//                     profile.emit(sender.clone(), Msg::FilterChanged);
//                 let mut platform = menu_choice("");
//                     platform.emit(sender.clone(), Msg::FilterChanged);
//                 let mut update = button::Button::default().with_label("\u{f0de}").with_size(20, 20);
//                     update.set_label_font(enums::Font::by_name("Wingdings-Regular"));
//                     update.set_label_size(20);
//                     update.set_color(Color::BackGround);
//                     update.set_selection_color(Color::BackGround);
//                     update.set_label_color(*systemGrayColor);
//                     update.set_frame(FrameType::OFlatFrame);
//                     update.emit(sender.clone(), Msg::UpdateSsh);
//                 filters.set_size(&profile, 70);
//                 filters.set_size(&platform, 70);
//                 filters.set_size(&update, 20);
//             filters.end();

//             let mut browser = browser::HoldBrowser::default();
//                 browser.style();
//                 browser.set_column_widths(&[300, 80]);
//                 browser.set_column_char('\t');
//                 browser.handle({
//                     let s = sender.clone();
//                     move |_, event| match event {
//                         Event::Released if app::event_clicks() => {
//                             s.send(Msg::Exec(Cmd::Ssh));
//                             true
//                         }
//                         _ => {
//                             s.send(Msg::HostChanged);
//                             false
//                         }
//                     }
//                 });

//             let mut buttons = group::Flex::default().row();
//                 main.set_size(&buttons, 30);
//                 let mut ssh = button("ssh");
//                 ssh.emit(sender.clone(), Msg::Exec(Cmd::Ssh));
//                 let mut commands = menu_button(&Cmd::iter().map(|x| x.as_ref().to_lowercase()).join("|"));
//                 commands.set_callback({
//                     let s = sender.clone();
//                     move |x| s.send(Msg::Exec(Cmd::from_repr(x.value() as usize).unwrap()))
//                 });
//                 buttons.set_size(&commands, 30);
//             buttons.end();

//         main.end();
//         win.end();
//         win.show();

//         let hosts = parse_ssh_config_from_host().unwrap_or_default();
//         let state = ExecOpt { hosts, ..ExecOpt::default() };
//         App { cfg, app, browser, ssh, commands, filter, profile, platform, state, sender, receiver, }
//     }

//     fn run(mut self) -> Result<()> {
//         self.sender.send(Msg::Load);
//         while self.app.wait() {
//             if let Some(msg) = self.receiver.recv() {
//                 match self.update(msg) {
//                     Ok(()) => {}
//                     Err(err) => p!("{err:?}"),
//                 }
//             }
//         }
//         Ok(())
//     }

//     fn update(&mut self, msg: Msg) -> Result<()> {
//         let matcher = SkimMatcherV2::default().ignore_case();
//         match msg {
//             Msg::Exec(cmd) if self.browser.selected_text().is_some() => {
//                 let sel = self.browser.selected_text().unwrap();
//                 let host = sel
//                     .split_once('\t')
//                     .map(fst)
//                     .ok_or_else(|| eyre!("Selected text is empty"))?
//                     .to_string();
//                 match cmd {
//                     Cmd::Ssh => Ssh::new(&ExecOpt { host, ..self.state.clone() })?.exec()?,
//                     Cmd::Rdp => {
//                         Tunnel::new("rdp", &ExecOpt { host, ..self.state.clone() }, &self.cfg)?
//                             .exec()?
//                     }
//                     Cmd::Code => Code::new(&ExecOpt { host, ..self.state.clone() })?.exec()?,
//                 }
//             }
//             Msg::Exec(_) => (),
//             Msg::HostChanged => {
//                 self.ssh.state(self.browser.selected_text().is_some());
//                 self.commands.state(self.browser.selected_text().is_some());
//             }
//             Msg::FilterChanged => {
//                 self.browser.clear();
//                 for (_, k, v) in self
//                     .state
//                     .hosts
//                     .iter()
//                     .sorted_by_key(|&(k, _)| k)
//                     .filter(|(_, h)| {
//                         if self.profile.value() == 0 {
//                             true
//                         } else {
//                             h.profile
//                                 == self.profile.text(self.profile.value()).expect("invalid profile")
//                         }
//                     })
//                     .filter(|(_, h)| {
//                         if self.platform.value() == 0 {
//                             true
//                         } else {
//                             h.platform
//                                 == self
//                                     .platform
//                                     .text(self.platform.value())
//                                     .expect("invalid platform")
//                         }
//                     })
//                     .filter_map(|(k, h @ Host { profile, .. })| {
//                         matcher
//                             .fuzzy_match(&f!("{profile}:{k}:{profile}"), self.filter.value().trim())
//                             .map(|score| (score, k, h))
//                     })
//                     .sorted_by_key(|&(score, _, _)| Reverse(score))
//                 {
//                     self.browser.add(&f!("{k}\t{}", v.profile));
//                 }
//                 if self.browser.size() > 0 {
//                     self.browser.select(1);
//                 }
//                 self.ssh.state(self.browser.selected_text().is_some());
//                 self.commands.state(self.browser.selected_text().is_some());
//             }
//             Msg::UpdateSsh => {
//                 update_sshconfig(
//                     &self.cfg.keys_path,
//                     &Config::template_path(),
//                     &self.cfg.bastion_name,
//                 )?;
//                 self.sender.send(Msg::Load)
//             }
//             Msg::Load => {
//                 let hosts = parse_ssh_config_from_host().unwrap_or_default();
//                 self.state.hosts = hosts;
//                 let profiles = ["all"]
//                     .into_iter()
//                     .chain(
//                         self.state.hosts.iter().map(|(_, h)| h.profile.as_str()).sorted().unique(),
//                     )
//                     .join("|");
//                 self.profile.clear();
//                 self.profile.add_choice(&profiles);
//                 self.profile.set_value(0);
//                 let platforms = ["all"]
//                     .into_iter()
//                     .chain(
//                         self.state.hosts.iter().map(|(_, h)| h.platform.as_str()).sorted().unique(),
//                     )
//                     .join("|");
//                 self.platform.clear();
//                 self.platform.add_choice(&platforms);
//                 self.platform.set_value(0);
//                 self.sender.send(Msg::FilterChanged);
//             }
//         }
//         Ok(())
//     }
// }

fn main() -> Result<()> {
    let options = eframe::NativeOptions::default();
    let state = MyApp::new()?;
    eframe::run_native("My egui App", options, Box::new(|_cc| Box::new(state)));
    Ok(())
}
