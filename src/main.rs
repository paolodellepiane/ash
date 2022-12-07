#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![warn(clippy::all)]
use aws::update_sshconfig;
use config::Config;
use eframe::epaint::TextShape;
use egui::{color::*, *};
use executable::*;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use parsers::ssh_config_parser::{parse_ssh_config_from_host, Host};
use poll_promise::Promise;
use prelude::*;
use std::{cmp::Reverse, collections::HashMap, str::FromStr};
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
    state: Promise<ExecOpt>,
    filter: String,
    profile: String,
    platform: String,
}

impl MyApp {
    fn filtered_hosts(&self) -> Vec<(i64, String, Host)> {
        let matcher = SkimMatcherV2::default().ignore_case();
        self.state
            .ready()
            .cloned()
            .unwrap_or_default()
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
            .chain(
                self.state
                    .ready()
                    .cloned()
                    .unwrap_or_default()
                    .hosts
                    .iter()
                    .map(|(_, h)| h.profile.as_str())
                    .sorted()
                    .unique(),
            )
            .map(String::from)
            .collect_vec()
    }

    fn platform(&self) -> Vec<String> {
        ["all"]
            .into_iter()
            .chain(
                self.state
                    .ready()
                    .cloned()
                    .unwrap_or_default()
                    .hosts
                    .iter()
                    .map(|(_, h)| h.platform.as_str())
                    .sorted()
                    .unique(),
            )
            .map(String::from)
            .collect_vec()
    }

    fn new(ctx: egui::Context) -> Result<Self> {
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
            state: Promise::from_ready(state),
            filter: Default::default(),
            platform: "all".into(),
            profile: "all".into(),
        })
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(self.state.ready().is_some(), |ui| {
                let mut style = (*ctx.style()).clone();
                ctx.set_style(style);
                ui.horizontal(|ui| {
                    TextEdit::singleline(&mut self.filter).hint_text("search").show(ui);
                    ComboBox::from_label("profile").selected_text(&self.profile).show_ui(
                        ui,
                        |ui| {
                            for p in self.profiles() {
                                ui.selectable_value(&mut self.profile, p.clone(), p);
                            }
                        },
                    );
                    ComboBox::from_label("platform").selected_text(&self.platform).show_ui(
                        ui,
                        |ui| {
                            for p in self.platform() {
                                ui.selectable_value(&mut self.platform, p.clone(), p);
                            }
                        },
                    );
                    match self.state.ready() {
                        Some(_) => {
                            if Button::new("🔃").fill(Color32::TRANSPARENT).ui(ui).clicked() {
                                self.state = Promise::spawn_thread("a", {
                                    let eo = self.state.ready().unwrap().clone();
                                    let cfg = self.cfg.clone();
                                    let ctx = ctx.clone();
                                    move || {
                                        update_sshconfig(&cfg);
                                        let hosts =
                                            parse_ssh_config_from_host().unwrap_or_default();
                                        ctx.request_repaint();
                                        ExecOpt { hosts, ..eo }
                                    }
                                });
                            }
                        }
                        _ => {
                            egui::Area::new("my_area")
                                .fixed_pos(egui::Pos2::ZERO)
                                .interactable(true)
                                .show(ctx, |ui| {
                                    ui.painter().rect_filled(
                                        ui.available_rect_before_wrap(),
                                        0.,
                                        Color32::from_rgba_unmultiplied(0, 0, 0, 150),
                                    );
                                    ui.centered_and_justified(|ui| ui.spinner());
                                });
                        }
                    }
                });
                ScrollArea::vertical().max_height(200.0).auto_shrink([false; 2]).show(ui, |ui| {
                    ui.vertical(|ui| {
                        for (_, k, v) in self.filtered_hosts() {
                            if ui
                                .selectable_value(
                                    &mut self.state.ready_mut().unwrap().host,
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
                                    Cmd::Ssh => Ssh::new(&self.state.ready().unwrap())
                                        .unwrap()
                                        .exec()
                                        .unwrap(),
                                    _ => (), // Cmd::Rdp => Tunnel::new("rdp", &self.state & self.cfg)?.exec()?,
                                             // Cmd::Code => {
                                             //     Code::new(&ExecOpt { host, ..self.state.clone() })?.exec()?
                                             // }
                                };
                                ui.close_menu();
                            }
                        }
                    });
                });
                ui.collapsing("ui", |ui| ctx.settings_ui(ui));
            });
        });
    }
}

fn main() -> Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| {
            let state = MyApp::new(_cc.egui_ctx.clone()).unwrap();

            Box::new(state)
        }),
    );
    Ok(())
}
