use crate::prelude::*;
use crate::teleport::{Host, Hosts};
use dialoguer::console::{Color, Style};
use dialoguer::theme::ColorfulTheme;
use dialoguer::FuzzySelect;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;
use std::process::exit;

pub fn select_str(message: &str, options: &Vec<String>, start_value: &str) -> Result<String> {
    let res = select(message, options, start_value)?;
    Ok(options[res].clone())
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
    pub hosts: Hosts,
    pub start_value: String,
}

pub fn select_teleport_host(SelectArgs { hosts, start_value }: &SelectArgs) -> Result<Host> {
    let width = hosts.iter().map(|x| x.spec.hostname.len()).max().unwrap_or(20);
    let values = hosts.iter().map(|h| f!("{:width$} [{h}]", h.spec.hostname.clone(),)).collect_vec();
    let idx = select("", &values, start_value)?;
    let selected = hosts.get(idx).unwrap();
    Ok(selected.clone())
}
