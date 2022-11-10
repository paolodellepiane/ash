#![warn(clippy::all)]
use aws::update_sshconfig;
use config::{Commands, CFG, Config};
use dialoguer::{
    console::{Color, Style},
    theme::ColorfulTheme,
    FuzzySelect,
};
use executable::*;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use parsers::ssh_config_parser::parse_ssh_config_from_host;
use prelude::*;

mod aws;
mod config;
mod describe_instances;
mod executable;
mod parsers;
mod prelude;

fn select(message: &str, options: Vec<String>, start_value: Option<String>) -> Result<String> {
    let matcher = SkimMatcherV2::default().ignore_case();
    if options.is_empty() {
        bail!("Host list is empty");
    }
    if let Some(sv) = start_value.clone() {
        let filtered =
            options.iter().filter_map(|x| matcher.fuzzy_match(x, &sv).map(|_| x)).collect_vec();
        if filtered.len() == 1 {
            return Ok(filtered[0].clone());
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
    let start_value = start_value.unwrap_or_default();
    let selection = FuzzySelect::with_theme(&theme)
        .with_prompt(message)
        .with_initial_text(start_value)
        .default(0)
        .items(&options)
        .interact()?;
    Ok(options[selection].clone())
}

fn select_profile_then_host(
    message: &str,
    Hosts { hosts, start_value, .. }: &Hosts,
) -> Result<String> {
    if CFG.0.merge_profiles {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        return select(message, values, start_value.clone());
    }
    let _select_profile_then_host = |(start_profile, start_host): (&str, &str)| {
        let profiles = hosts.iter().map(|(_, h)| h.profile.clone()).unique().collect_vec();
        let profile = select(
            "Choose Profile...",
            profiles,
            Some(start_profile.to_string()),
        )?;
        let values = hosts
            .iter()
            .filter_map(|(_, h)| (h.profile == profile).then_some(h.name.clone()))
            .collect_vec();
        select(message, values, Some(start_host.to_string()))
    };
    match start_value.as_deref() {
        Some(start_value) if start_value.contains(':') => {
            _select_profile_then_host(start_value.split_once(':').unwrap())
        }
        Some(_) => {
            let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
            select(message, values, start_value.clone())
        }
        None => _select_profile_then_host(("", "")),
    }
}

fn run() -> Result<()> {
    let (config, args) = &*CFG;
    if config.update {
        update_sshconfig(
            &config.keys_path,
            &Config::template_path(),
            config.bastion_name.as_deref(),
        )?;
    }
    let hosts = parse_ssh_config_from_host()?;
    let hosts =
        &Hosts { hosts, start_value: args.host.clone(), bastion: config.bastion_name.clone() };
    match &args.command {
        Some(Commands::Cp(cp)) => Scp::new(cp, hosts)?.exec(),
        Some(Commands::Service { service }) => Tunnel::from_service(service, hosts)?.exec(),
        Some(Commands::Tunnel(tunnel)) => Tunnel::from_ports(*tunnel, hosts)?.exec(),
        Some(Commands::Exec { command }) => Exec::new(command, hosts)?.exec(),
        Some(Commands::Code) => Code::new(hosts)?.exec(),
        None => Ssh::new(hosts)?.exec(),
    }
}

fn main() -> Result<()> {
    run()?;
    Ok(())
}
