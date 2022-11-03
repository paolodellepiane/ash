use crate::config::Service;
use crate::config::COMMON_SSH_ARGS;
use crate::prelude::*;
use crate::select;
use crate::Host;
use crate::OptionNotEmptyString;
use clap::Args;
use clap::arg;
use itertools::Itertools;
use std::collections::HashMap;
use std::process::Command;

pub trait Executable {
    fn exec(&self) -> Result<()>;
}

pub struct Hosts {
    pub hosts: HashMap<String, Host>,
    pub start_value: OptionNotEmptyString,
    pub bastion: OptionNotEmptyString,
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
        Hosts { hosts, start_value, bastion }: &Hosts,
    ) -> Result<Self> {
        if bastion.is_none() {
            bail!("Can't tunnel without bastion");
        }
        let bastion = hosts
            .get(bastion.as_deref().unwrap())
            .ok_or_else(|| anyhow!("Can't find bastion {bastion:?}"))?
            .clone();
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        let choice = select("Tunnel to...", values, start_value)?;
        let host = hosts[&choice].clone();

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
    pub to: String,
}

#[derive(Clone)]
pub struct Scp {
    from: String,
    to: String,
}

impl Scp {
    pub fn new(ScpArgs { from, to }: &ScpArgs, Hosts { hosts, .. }: &Hosts) -> Result<Self> {
        fn expand_remote(s: &str, hosts: &HashMap<String, Host>) -> Result<(String, Option<Host>)> {
            if let Some((srv, path)) = s.split_once(':') {
                let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
                let choice = select("Select Host...", values, &srv.into())?;
                let host @ Host { name, .. } = &hosts[&choice];
                let res = f!("{name}:{path}");
                Ok((res, Some(host.clone())))
            } else {
                Ok((String::from(s), None))
            }
        }
        if from.contains(':') && to.contains(':') {
            bail!("Both 'From' and 'To' contain ':'. Use ':' for remote host only")
        }
        if !from.contains(':') && !to.contains(':') {
            bail!("Either 'From' or 'To' must contain ':'. Use ':' for remote host only")
        }
        let (from, from_host) = expand_remote(from, hosts)?;
        let (to, to_host) = expand_remote(to, hosts)?;
        from_host.or(to_host).expect("No host found");

        Ok(Self { from, to })
    }
}

impl Executable for Scp {
    fn exec(&self) -> Result<()> {
        let Self { from, to, .. } = self;
        p!("Copying from {from} to {to}...");
        Command::new("scp").args(COMMON_SSH_ARGS).args([from, to]).status()?;

        Ok(())
    }
}

pub struct Ssh {
    host: Host,
}

impl Ssh {
    pub fn new(Hosts { hosts, start_value, .. }: &Hosts) -> Result<Self> {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        let choice = select("Connect to Host...", values, start_value)?;

        Ok(Self { host: hosts[&choice].clone() })
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
    pub fn new(command: &str, Hosts { hosts, start_value, .. }: &Hosts) -> Result<Self> {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        let choice = select("Execute on Host...", values, start_value)?;

        Ok(Self { host: hosts[&choice].clone(), command: command.to_string() })
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
