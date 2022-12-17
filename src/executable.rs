use crate::config::Config;
use crate::config::Service;
use crate::config::COMMON_SSH_ARGS;
use crate::parsers::ssh_config_parser::Host;
use crate::prelude::*;
use crate::{select, select_profile_then_host};
use clap::arg;
use clap::Args;
use itertools::Itertools;
use std::collections::HashMap;
use std::process::Command;

pub trait Executable {
    fn exec(&self) -> Result<()>;
}

pub struct Hosts {
    pub hosts: HashMap<String, Host>,
    pub start_value: String,
    pub bastion: String,
}

#[derive(Args, Clone, Copy)]
pub struct TunnelArgs {
    /// Local port
    local: u16,
    /// Remote port
    remote: u16,
}

#[derive(Clone)]
pub struct Tunnel {
    local: u16,
    remote: u16,
    host: Host,
    bastion: Host,
}

impl Tunnel {
    pub fn from_ports(
        TunnelArgs { local, remote }: TunnelArgs,
        hosts @ Hosts { bastion, .. }: &Hosts,
    ) -> Result<Self> {
        if bastion.is_empty() {
            bail!("Can't tunnel without bastion");
        }
        let bastion = hosts
            .hosts
            .get(bastion)
            .ok_or_else(|| eyre!("Can't find bastion {bastion:?}"))?
            .clone();
        let choice = select_profile_then_host(hosts)?;
        let host = hosts.hosts[&choice].clone();
        Ok(Self { local, remote, host, bastion })
    }

    pub fn from_service(service: &Service, hosts: &Hosts) -> Result<Self> {
        let (local, remote) = match service {
            Service::Rdp => (3389, 3389),
            Service::Redis => (6379, 6379),
            Service::Rds => (5432, 5432),
            Service::RabbitMq => (5672, 5672),
        };
        Tunnel::from_ports(TunnelArgs { local, remote }, hosts)
    }
}

impl Executable for Tunnel {
    fn exec(&self) -> Result<()> {
        let Self {
            local,
            remote,
            host: Host { name, address, .. },
            bastion: Host { name: bastion_name, .. },
        } = self;
        p!("Tunneling from {local} to {name}:{remote} through {bastion_name} ...");
        Command::new("ssh")
            .args(COMMON_SSH_ARGS)
            .args(["-N", "-L", &f!("{local}:{address}:{remote}"), bastion_name])
            .status()?;

        Ok(())
    }
}

#[derive(Args)]
pub struct ScpArgs {
    /// From    (use ':' to copy from remote, e.g. 'ash cp <remote>:fake.toml .')
    #[arg(long_help("use ':' to copy from remote, e.g.:\n'ash cp <remote>:fake.toml .' : copy fake:toml from <remote> to current dir\n<remote> can be empty or partial, ash will ask to select it from a list"))]
    pub from: String,
    /// To    (use ':' to copy to remote, e.g. 'ash cp fake.toml <remote>:fake.toml')
    #[arg(long_help("use ':' to copy to remote, e.g.:\n'ash cp fake.toml <remote>:fake.toml .' : copy fake:toml from current dir to <remote>\n<remote> can be empty or partial, ash will ask to select it from a list"))]
    pub to: Option<String>,
}

#[derive(Clone)]
pub struct Scp {
    from: String,
    to: String,
}

impl Scp {
    pub fn new(ScpArgs { from, to }: &ScpArgs, hosts: &Hosts) -> Result<Self> {
        fn expand_remote(s: &str, hosts: &Hosts, is_from: bool) -> Result<(String, Option<Host>)> {
            if let Some((start_value, path)) = s.rsplit_once(':') {
                if is_from && path.is_empty() {
                    bail!("FROM must contain a path to file or folder")
                }
                let hosts = &Hosts {
                    start_value: start_value.to_string(),
                    hosts: hosts.hosts.clone(),
                    bastion: String::new(),
                };
                let choice = select_profile_then_host(hosts)?;
                let host @ Host { name, .. } = &hosts.hosts[&choice];
                let res = f!("{name}:{path}");
                Ok((res, Some(host.clone())))
            } else {
                Ok((String::from(s), None))
            }
        }
        let mut to = to.to_owned().unwrap_or_default();
        if to.is_empty() {
            to = if from.contains(':') { "." } else { ":" }.to_owned() // want to copy from remote to local else from local to remote
        }
        if from.contains(':') && to.contains(':') {
            bail!("Both 'From' and 'To' contain ':'. Use ':' for remote host only")
        }
        if !from.contains(':') && !to.contains(':') {
            bail!("Either 'From' or 'To' must contain ':'. Use ':' for remote host only")
        }
        let (from, from_host) = expand_remote(from, hosts, true)?;
        let (to, to_host) = expand_remote(&to, hosts, false)?;
        from_host.or(to_host).context("No host found")?;
        Ok(Self { from, to })
    }
}

impl Executable for Scp {
    fn exec(&self) -> Result<()> {
        let Self { from, to, .. } = self;
        p!("Copying from {from} to {to}...");
        Command::new("scp").args(COMMON_SSH_ARGS).args(["-r", from, to]).status()?;
        Ok(())
    }
}

pub struct Ssh {
    host: Host,
}

impl Ssh {
    pub fn new(hosts: &Hosts) -> Result<Self> {
        let choice = select_profile_then_host(hosts)?;
        Ok(Self { host: hosts.hosts[&choice].clone() })
    }
}

impl Executable for Ssh {
    fn exec(&self) -> Result<()> {
        let Host { name, .. } = &self.host;
        p!("Connecting to {name}...");
        Command::new("ssh").args(COMMON_SSH_ARGS).arg(name).status()?;
        Ok(())
    }
}

pub struct Exec {
    host: Host,
    command: String,
}

impl Exec {
    pub fn new(command: &str, hosts: &Hosts) -> Result<Self> {
        let choice = select_profile_then_host(hosts)?;
        Ok(Self { host: hosts.hosts[&choice].clone(), command: command.to_string() })
    }
}

impl Executable for Exec {
    fn exec(&self) -> Result<()> {
        let Self { command, host: Host { name, .. } } = self;
        p!("Executing on {name}...");
        Command::new("ssh").args(COMMON_SSH_ARGS).args([name, command]).status()?;
        Ok(())
    }
}

pub struct Code {
    host: Host,
}

impl Code {
    pub fn new(hosts: &Hosts) -> Result<Self> {
        let choice = select_profile_then_host(hosts)?;
        Ok(Self { host: hosts.hosts[&choice].clone() })
    }
}

impl Executable for Code {
    fn exec(&self) -> Result<()> {
        let Self { host: Host { name, .. } } = self;
        p!("Connect vscode to remote host {name}...");
        Command::new(Config::code_cmd())
            .args(["--folder-uri", &f!("vscode-remote://ssh-remote+{name}/")])
            .status()?;
        Ok(())
    }
}

pub struct Info {
    host: Host,
}

impl Info {
    pub fn new(hosts: &Hosts) -> Result<Self> {
        let choice = select_profile_then_host(hosts)?;
        Ok(Self { host: hosts.hosts[&choice].clone() })
    }
}

impl Executable for Info {
    fn exec(&self) -> Result<()> {
        let host = serde_json::to_string_pretty(&self.host)?;
        p!("{host}");
        Ok(())
    }
}

pub struct Vsdbg {
    host: Host,
}

impl Vsdbg {
    pub fn new(hosts: &Hosts) -> Result<Self> {
        let choice = select_profile_then_host(hosts)?;
        let name = &hosts.hosts[&choice].name;
        let res = Command::new("ssh")
            .args(COMMON_SSH_ARGS)
            .args([
                name,
                r#"sudo docker ps --format "{{.ID}},{{.Names}},{{.Image}}""#,
            ])
            .output()?
            .stdout;
        let res = String::from_utf8_lossy(&res).into_owned();
        let containers: HashMap<_, _> = res
            .lines()
            .map(|l| l.split(',').collect_vec())
            .filter(|s| s.len() > 1)
            .map(|s| (f!("{} - {} - {}", s[0], s[1], s[2]), s[0]))
            .collect();
        let container = select("", containers.clone().into_keys().collect_vec(), "")?;
        // executeInteractive(`scp`, `-i`, s.Key, lookForPath(`res/vsdbg.sh`, vsdbgsh), s.Address+`:`)
        // executeInteractive(`ssh`, `-i`, s.Key, s.Address, `sudo`, `bash`, `vsdbg.sh`, containers[i][0], *vsdbgPortFlag)
        p!("selected {}", containers[&container]);
        Ok(Self { host: hosts.hosts[&choice].clone() })
    }
}

impl Executable for Vsdbg {
    fn exec(&self) -> Result<()> {
        let host = serde_json::to_string_pretty(&self.host)?;
        p!("{host}");
        Ok(())
    }
}
