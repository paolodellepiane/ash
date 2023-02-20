use crate::config::Config;
use crate::prelude::*;
use crate::teleport::{Welcome, WelcomeElement};
use dialoguer::console::{Color, Style};
use dialoguer::theme::ColorfulTheme;
use dialoguer::FuzzySelect;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::process::exit;

#[derive(Serialize, Deserialize, Default)]
pub struct History {
    entries: Vec<WelcomeElement>,
}

impl History {
    pub fn load() -> Self {
        let path = Config::history_path();
        if !path.exists() {
            History { ..Default::default() }.save();
        }
        let h = File::open(path).expect("can't load history");
        serde_json::from_reader(h).expect("Error deserializing history")
    }

    pub fn update(host: &WelcomeElement) {
        let mut h = Self::load();
        h.entries.retain(|x| x.metadata.name != host.metadata.name);
        h.entries.insert(0, host.to_owned());
        h.save();
    }

    pub fn intersect(hosts: &Welcome) {
        let mut h = Self::load();
        h.entries.retain(|x| hosts.iter().any(|y| y.metadata.name == x.metadata.name));
        h.save();
    }

    fn save(&self) {
        std::fs::write(Config::history_path(), serde_json::to_string(self).unwrap()).unwrap();
    }
}

pub fn select(message: &str, options: &Vec<String>, start_value: &str) -> Result<usize> {
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

pub struct SelectArgs {
    pub hosts: Welcome,
    pub start_value: String,
}

pub fn select_teleport_host(SelectArgs { mut hosts, ref start_value }: SelectArgs) -> Result<WelcomeElement> {
    History::intersect(&hosts);
    let recents = History::load().entries;
    let width = hosts.iter().map(|x| x.spec.hostname.len()).max().unwrap_or(20);
    hosts.retain(|x| !recents.contains(x));
    let hosts = [recents, hosts].concat();
    let values = hosts.iter().map(|h| f!("{:width$} [{h}]", h.spec.hostname.clone(),)).collect_vec();
    let idx = select("", &values, start_value)?;
    let selected = hosts.get(idx).unwrap();
    History::update(selected);
    Ok(selected.clone())
}
