#![warn(clippy::all)]
use aws::update_sshconfig;
use config::{Commands, CFG};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, console::{Style, Color}};
use executable::*;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use itertools::Itertools;
use prelude::*;
use ssh_config_parser::parse_host_ssh_config;

mod aws;
mod config;
mod describe_instances;
mod executable;
mod prelude;
mod ssh_config_parser;

fn select(message: &str, options: Vec<String>, start_value: Option<String>) -> Result<String> {
    let matcher = SkimMatcherV2::default().ignore_case();
    if let Some(sv) = start_value {
        let scores = options
            .iter()
            .enumerate()
            .filter_map(|(i, x)| {
                matcher.fuzzy_match(&x, &sv).map(|score| (i, score))
            })
            .sorted_by_key(|x| x.1)
            .rev()
            .collect_vec();
        if scores.len() == 1 {
            return Ok(options[scores[0].0].clone());
        }
        if scores.is_empty() {
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
            &config.template_file_path,
            config.bastion_name.as_deref(),
        )?;
    }
    let hosts = parse_host_ssh_config()?;
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
