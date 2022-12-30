#![warn(clippy::all)]
use aws::update_sshconfig;
use commands::*;
use config::{Config, CFG};
use dialoguer::{
    console::{Color, Style},
    theme::ColorfulTheme,
    FuzzySelect,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use parsers::ssh_config_parser::parse_ssh_config_from_host;
use prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::exit};
use std::{fs::File, iter::once};

mod aws;
mod commands;
mod config;
mod describe_instances;
mod parsers;
mod prelude;
mod ssh;

fn select_idx(message: &str, options: &Vec<String>, start_value: &str) -> Result<usize> {
    let matcher = SkimMatcherV2::default().ignore_case();
    if options.is_empty() {
        bail!("Host list is empty");
    }
    if !start_value.is_empty() {
        let filtered = options
            .iter()
            .enumerate()
            .filter_map(|(i, x)| matcher.fuzzy_match(x, start_value).map(|_| (i, x)))
            .collect_vec();
        if filtered.len() == 1 {
            return Ok(filtered[0].0);
        }
        if filtered.is_empty() {
            bail!("No host found");
        }
    }
    let theme = ColorfulTheme {
        active_item_style: Style::new().fg(Color::Green),
        fuzzy_match_highlight_style: Style::new().fg(Color::Green),
        ..ColorfulTheme::default()
    };
    let selection = FuzzySelect::with_theme(&theme)
        .with_prompt(message)
        .with_initial_text(start_value)
        .default(0)
        .items(options)
        .interact_opt()?
        .unwrap_or_else(|| exit(0));
    Ok(selection)
}

fn select(message: &str, options: &Vec<String>, start_value: &str) -> Result<String> {
    let idx = select_idx(message, options, start_value)?;
    Ok(options.get(idx).unwrap().clone())
}

#[derive(Serialize, Deserialize, Default)]
struct History {
    entries: Vec<String>,
}

impl History {
    pub fn load() -> Self {
        let path = Config::config_dir().join("history");
        if !path.exists() {
            History { ..Default::default() }.save();
        }
        let h = File::open(path).expect("can't load history");
        serde_json::from_reader(h).expect("Error deserializing history")
    }

    pub fn update(entry: &str) {
        let mut h = Self::load();
        h.entries.retain(|x| x != entry);
        h.entries.insert(0, entry.to_owned());
        h.save();
    }

    pub fn intersect(hosts: &HashMap<String, parsers::ssh_config_parser::Host>) {
        let mut h = Self::load();
        let mut iter = hosts.keys();
        h.entries.retain(|x| iter.any(|y| y == x));
        h.save();
    }

    fn save(&self) {
        std::fs::write(
            Config::config_dir().join("history"),
            serde_json::to_string(self).unwrap(),
        )
        .unwrap();
    }
}

fn select_profile_then_host(Hosts { hosts, start_value, .. }: &Hosts) -> Result<String> {
    if CFG.0.merge_profiles {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        let selected = select("", &values, start_value);
        if let Ok(s) = &selected {
            History::update(s);
        }
        return selected;
    }
    let _select_profile_then_host = |(start_profile, start_host): (&str, &str)| {
        let profiles = hosts.iter().map(|(_, h)| h.profile.clone()).unique();
        let profiles = once("history".to_string()).chain(profiles).collect_vec();
        let profile = select("", &profiles, start_profile)?;
        let values = hosts
            .iter()
            .filter_map(|(_, h)| (h.profile == profile).then_some(h.name.clone()))
            .collect_vec();
        let selected = if profile == "history" {
            select(&f!("[{profile}]"), &History::load().entries, start_host)
        } else {
            select(&f!("[{profile}]"), &values, start_host)
        };
        if let Ok(s) = &selected {
            History::update(s);
        }
        selected
    };
    match start_value {
        sv if sv.contains(':') => _select_profile_then_host(sv.split_once(':').unwrap()),
        sv if sv.is_empty() => _select_profile_then_host(("", "")),
        _ => {
            let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
            select("", &values, start_value)
        }
    }
}

fn run() -> Result<()> {
    let (config, args) = &*CFG;
    if args.check_update {
        if cfg!(windows) {
            std::process::Command::new("scoop.cmd").args(["update", "-k", "ash"]).spawn()?;
        } else {
            p!("not implemented on this platform");
        }
        exit(0);
    }
    if config.update {
        update_sshconfig(
            &config.keys_path,
            Config::template_path(),
            &config.bastion_name,
        )?;
    }
    let hosts = parse_ssh_config_from_host()?;
    History::intersect(&hosts);
    let hosts = &Hosts {
        hosts,
        start_value: args.host.clone().unwrap_or_default(),
        bastion: config.bastion_name.clone(),
    };
    use Commands::*;
    match &args.command {
        Some(cmd) => match cmd {
            Cp(cp) => Commands::cp(cp, hosts),
            Service { service } => Commands::tunnel_from_service(service, hosts),
            Tunnel(tunnel) => Commands::tunnel_from_ports(*tunnel, hosts),
            Exec { command } => Commands::exec(command, hosts),
            Code => Commands::code(hosts),
            Info => Commands::info(hosts),
            Vsdbg => Commands::vsdbg(hosts),
            EventLog => Commands::win_event_log(hosts),
            ContainerEventLog => Commands::win_container_event_log(hosts),
            Get => Commands::get_file(hosts),
            Put => Commands::put_file(hosts),
        },
        None => Commands::ssh(hosts),
    }
}

fn main() -> Result<()> {
    run()
}
