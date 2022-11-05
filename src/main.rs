#![warn(clippy::all)]
use aws::update_sshconfig;
use config::{Commands, Config, CFG};
use executable::{Exec, Executable, Hosts, Scp, Ssh, Tunnel};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use inquire::{InquireError, Select};
use itertools::Itertools;
use prelude::*;
use serde::Serialize;
use std::{collections::HashMap, fmt::Debug};

mod aws;
mod config;
mod describe_instances;
mod executable;
mod prelude;

#[derive(Clone, Debug, Serialize)]
pub struct Host {
    pub name: String,
    pub profile: String,
    pub address: String,
    pub user: String,
    pub key: Option<String>,
}

fn parse_hosts() -> Result<HashMap<String, Host>> {
    const HOSTS: &str = r"(?smU)# generated\s*\[(?P<profile>\S*)\]\s*?$.*Host\s*(?P<name>\S*)\s*$\s*HostName\s*(?P<address>\S*)\s*$\s*User\s*(?P<user>\S*)\s*$\s*IdentityFile\s*(?P<key>\S*)\s*$";
    let ssh_config_path = Config::home_dir().join(".ssh").join("config");
    let ssh_config = std::fs::read_to_string(ssh_config_path)?;
    // let _guard = stopwatch("ssh parse");
    let res: HashMap<_, _> = regex::Regex::new(HOSTS)?
        .captures_iter(&ssh_config)
        .filter_map(|c| {
            let profile = c.name("profile")?.as_str().to_string();
            let name = c.name("name")?.as_str().to_string();
            let address = c.name("address")?.as_str().to_string();
            let user = c.name("user")?.as_str().to_string();
            let key = c.name("key").map(|x| String::from(x.as_str()));
            Some((name.clone(), Host { name, profile, address, user, key }))
        })
        .collect();
    Ok(res)
}

fn select(message: &str, options: Vec<String>, start_value: Option<String>) -> Result<String> {
    let matcher = SkimMatcherV2::default().ignore_case();
    let options = options
        .into_iter()
        .filter(|x| {
            start_value.as_deref().map_or(true, |filter| matcher.fuzzy_match(x, filter).is_some())
        })
        .sorted()
        .collect_vec();
    if options.is_empty() {
        bail!("No host found");
    }
    if options.len() == 1 && start_value.is_some() {
        return Ok(options[0].clone());
    }
    let ans = Select::new(message, options)
        .with_filter(&|filter: &str, _, string_value: &str, _| -> bool {
            matcher.fuzzy_match(string_value, filter).is_some()
        })
        .prompt()?;

    Ok(ans)
}

fn select_profile_then_host(
    message: &str,
    Hosts { hosts, start_value, .. }: &Hosts,
) -> Result<String> {
    if CFG.0.merge_profiles {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        return select(message, values, start_value.clone())
    }
    let _select_profile_then_host = |(start_profile, start_host): (&str, &str)| {
        let profiles = hosts.iter().map(|(_, h)| h.profile.clone()).unique().collect_vec();
        let profile = select("Choose Profile...", profiles, Some(start_profile.to_string()))?;
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
    let hosts = parse_hosts()?;
    let hosts = &Hosts {
        hosts,
        start_value: args.host.clone(),
        bastion: config.bastion_name.clone(),
    };
    match &args.command {
        Some(Commands::Cp(cp)) => Scp::new(cp, hosts)?.exec(),
        Some(Commands::Service { service }) => Tunnel::from_service(service, hosts)?.exec(),
        Some(Commands::Tunnel(tunnel)) => Tunnel::from_ports(*tunnel, hosts)?.exec(),
        Some(Commands::Exec { command }) => Exec::new(command, hosts)?.exec(),
        None => Ssh::new(hosts)?.exec(),
    }
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        match err.downcast_ref::<InquireError>() {
            None => bail!(err),
            Some(_) => (),
        }
    }
    Ok(())
}
