#![warn(clippy::all)]
use aws::update_sshconfig;
use config::{Commands, Config};
use executable::{Exec, Executable, Hosts, Scp, Ssh, Tunnel};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use inquire::{InquireError, Select};
use itertools::Itertools;
use pollster::FutureExt;
use prelude::*;
use serde::Serialize;
use ssh_cfg::{SshConfigParser, SshOptionKey};
use std::{collections::HashMap, fmt::Debug};

mod aws;
mod config;
mod describe_instances;
mod executable;
mod option_not_empty_string;
mod prelude;

#[derive(Clone, Debug, Serialize)]
pub struct Host {
    pub name: String,
    pub address: String,
    pub user: String,
    pub key: Option<String>,
}

async fn parse_hosts() -> Result<HashMap<String, Host>> {
    let ssh_config = SshConfigParser::parse_home().await?;
    let res = ssh_config.iter()
                        .filter_map(|(h, c)| {
                            let i = Host { name: h.clone(),
                                           address: c.get(&SshOptionKey::Hostname)?.clone(),
                                           user: c.get(&SshOptionKey::User)?.clone(),
                                           key: c.get(&SshOptionKey::IdentityFile)
                                                 .map(String::from) };
                            Some(i)
                        })
                        .map(|x| (x.name.clone(), x))
                        .collect();

    Ok(res)
}

fn select(message: &str,
          options: Vec<String>,
          start_value: &OptionNotEmptyString)
          -> Result<String> {
    let matcher = SkimMatcherV2::default().ignore_case();
    let options = options.into_iter()
                         .filter(|x| {
                             start_value.as_deref().map_or(true, |filter| {
                                                       matcher.fuzzy_match(x, filter).is_some()
                                                   })
                         })
                         .sorted()
                         .collect_vec();
    if options.is_empty() {
        bail!("No host found");
    }
    if options.len() == 1 {
        return Ok(options[0].clone());
    }
    let ans =
        Select::new(message, options).with_filter(&|filter: &str,
                                                    _,
                                                    string_value: &str,
                                                    _|
                                      -> bool {
                                         matcher.fuzzy_match(string_value, filter).is_some()
                                     })
                                     .prompt()?;

    Ok(ans)
}

async fn run() -> Result<()> {
    let (config, args) = &mut Config::load()?;
    if config.update {
        update_sshconfig(&config.keys_path,
                         &config.template_file_path,
                         config.bastion_name.as_deref())?;
    }

    let hosts = parse_hosts().await?;
    let hosts = &Hosts { hosts,
                         start_value: args.host.clone().into(),
                         bastion: config.bastion_name.clone().into() };

    match &args.command {
        Some(Commands::Cp(cp)) => Scp::new(cp, hosts)?.exec(),
        Some(Commands::Service { service }) => Tunnel::from_service(service, hosts)?.exec(),
        Some(Commands::Tunnel(tunnel)) => Tunnel::from_ports(*tunnel, hosts)?.exec(),
        Some(Commands::Exec { command }) => Exec::new(command, hosts)?.exec(),
        None => Ssh::new(hosts)?.exec(),
    }
}

fn main() -> Result<()> {
    if let Err(err) = run().block_on() {
        match err.downcast_ref::<InquireError>() {
            None => bail!(err),
            Some(_) => (),
        }
    }
    Ok(())
}
