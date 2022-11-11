#![warn(clippy::all)]
use aws::update_sshconfig;
use color_eyre::config::HookBuilder;
use config::{Commands, Config, CFG};
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

fn select(message: &str, options: Vec<String>, start_value: &str) -> Result<String> {
    let matcher = SkimMatcherV2::default().ignore_case();
    if options.is_empty() {
        bail!("Host list is empty");
    }
    if !start_value.is_empty() {
        let filtered = options
            .iter()
            .filter_map(|x| matcher.fuzzy_match(x, start_value).map(|_| x))
            .collect_vec();
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
    let selection = FuzzySelect::with_theme(&theme)
        .with_prompt(message)
        .with_initial_text(start_value)
        .default(0)
        .items(&options)
        .interact()?;
    Ok(options[selection].clone())
}

fn select_profile_then_host(Hosts { hosts, start_value, .. }: &Hosts) -> Result<String> {
    if CFG.0.merge_profiles {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        return select("", values, start_value);
    }
    let _select_profile_then_host = |(start_profile, start_host): (&str, &str)| {
        let profiles = hosts.iter().map(|(_, h)| h.profile.clone()).unique().collect_vec();
        let profile = select("", profiles, start_profile)?;
        let values = hosts
            .iter()
            .filter_map(|(_, h)| (h.profile == profile).then_some(h.name.clone()))
            .collect_vec();
        select(&f!("[{profile}]"), values, start_host)
    };
    match start_value {
        sv if sv.contains(':') => _select_profile_then_host(sv.split_once(':').unwrap()),
        sv if sv.is_empty() => _select_profile_then_host(("", "")),
        _ => {
            let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
            select("", values, start_value)
        }
    }
}

fn run() -> Result<()> {
    let (config, args) = &*CFG;
    if config.update {
        update_sshconfig(
            &config.keys_path,
            &Config::template_path(),
            &config.bastion_name,
        )?;
    }
    let hosts = parse_ssh_config_from_host()?;
    let hosts = &Hosts {
        hosts,
        start_value: args.host.clone().unwrap_or_default(),
        bastion: config.bastion_name.clone(),
    };
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
    HookBuilder::default().display_env_section(false).install()?;
    run()?;
    Ok(())
}
