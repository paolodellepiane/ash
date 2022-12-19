use crate::config::Config;
use crate::config::Service;
use crate::config::COMMON_SSH_ARGS;
use crate::parsers::ssh_config_parser::Host;
use crate::prelude::*;
use crate::select_idx;
use crate::select_profile_then_host;
use clap::arg;
use clap::command;
use clap::Args;
use clap::Subcommand;
use itertools::Itertools;
use std::collections::HashMap;
use std::process::Command;

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

#[derive(Args)]
pub struct ScpArgs {
    /// From    (use ':' to copy from remote, e.g. 'ash cp <remote>:fake.toml .')
    #[arg(long_help("use ':' to copy from remote, e.g.:\n'ash cp <remote>:fake.toml .' : copy fake:toml from <remote> to current dir\n<remote> can be empty or partial, ash will ask to select it from a list"))]
    pub from: String,
    /// To    (use ':' to copy to remote, e.g. 'ash cp fake.toml <remote>:fake.toml')
    #[arg(long_help("use ':' to copy to remote, e.g.:\n'ash cp fake.toml <remote>:fake.toml .' : copy fake:toml from current dir to <remote>\n<remote> can be empty or partial, ash will ask to select it from a list"))]
    pub to: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Copy file/folder from remote
    #[command(arg_required_else_help = false, after_help("Folder path not ending with '/' will copy the directory including contents, rather than only the contents of the directory"))]
    Cp(ScpArgs),
    /// Create a tunnel for a predefined service
    #[command(arg_required_else_help = true)]
    Service {
        /// Common Services
        service: Service,
    },
    /// Create a tunnel for custom ports
    #[command(arg_required_else_help = true)]
    Tunnel(TunnelArgs),
    /// Execute a command remotely
    #[command(arg_required_else_help = true)]
    Exec {
        /// Command to execute
        command: String,
    },
    /// Connect vscode to remote host
    #[command()]
    Code,
    /// Output selected host info
    #[command()]
    Info,
    /// Try to setup remote container for remote debug
    #[command()]
    Vsdbg,
}

impl Commands {
    pub fn tunnel_from_ports(
        TunnelArgs { local, remote }: TunnelArgs,
        hosts @ Hosts { bastion, .. }: &Hosts,
    ) -> Result<()> {
        if bastion.is_empty() {
            bail!("Can't tunnel without bastion");
        }
        let bastion = hosts
            .hosts
            .get(bastion)
            .ok_or_else(|| eyre!("Can't find bastion {bastion:?}"))?
            .clone();
        let bastion_name = &bastion.name;
        let choice = select_profile_then_host(hosts)?;
        let Host { name, address, .. } = &hosts.hosts[&choice];
        p!("Tunneling from {local} to {name}:{remote} through {bastion_name} ...");
        Command::new("ssh")
            .args(COMMON_SSH_ARGS)
            .args(["-N", "-L", &f!("{local}:{address}:{remote}"), bastion_name])
            .status()?;

        Ok(())
    }

    pub fn tunnel_from_service(service: &Service, hosts: &Hosts) -> Result<()> {
        let (local, remote) = match service {
            Service::Rdp => (3389, 3389),
            Service::Redis => (6379, 6379),
            Service::Rds => (5432, 5432),
            Service::RabbitMq => (5672, 5672),
        };
        Self::tunnel_from_ports(TunnelArgs { local, remote }, hosts)
    }

    pub fn cp(ScpArgs { from, to }: &ScpArgs, hosts: &Hosts) -> Result<()> {
        fn expand_remote(s: &str, hosts: &Hosts, is_from: bool) -> Result<String> {
            if let Some((start_value, path)) = s.rsplit_once(':') {
                if is_from && path.is_empty() {
                    bail!("FROM must contain a path to file or folder")
                }
                let hosts = &Hosts {
                    start_value: start_value.to_string(),
                    hosts: hosts.hosts.clone(),
                    bastion: String::new(),
                };
                let name = select_profile_then_host(hosts)?;
                let res = f!("{name}:{path}");
                Ok(res)
            } else {
                Ok(String::from(s))
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
        let from = expand_remote(from, hosts, true)?;
        let to = expand_remote(&to, hosts, false)?;
        p!("Copying from {from} to {to}...");
        Command::new("scp").args(COMMON_SSH_ARGS).args(["-r", &from, &to]).status()?;
        Ok(())
    }

    pub fn ssh(hosts: &Hosts) -> Result<()> {
        let name = &select_profile_then_host(hosts)?;
        p!("Connecting to {name}...");
        Command::new("ssh").args(COMMON_SSH_ARGS).arg(name).status()?;
        Ok(())
    }

    pub fn exec(command: &str, hosts: &Hosts) -> Result<()> {
        let name = &select_profile_then_host(hosts)?;
        p!("Executing on {name}...");
        Command::new("ssh").args(COMMON_SSH_ARGS).args([name, command]).status()?;
        Ok(())
    }

    pub fn code(hosts: &Hosts) -> Result<()> {
        let name = &select_profile_then_host(hosts)?;
        p!("Connect vscode to remote host {name}...");
        Command::new(Config::code_cmd())
            .args(["--folder-uri", &f!("vscode-remote://ssh-remote+{name}/")])
            .status()?;
        Ok(())
    }

    pub fn info(hosts: &Hosts) -> Result<()> {
        let choice = select_profile_then_host(hosts)?;
        let host = serde_json::to_string_pretty(&hosts.hosts[&choice])?;
        p!("{host}");
        Ok(())
    }

    pub fn vsdbg(hosts: &Hosts) -> Result<()> {
        let host_name = &select_profile_then_host(hosts)?;
        let res = ssh_execute(
            host_name,
            r#"sudo docker ps --format "{{.ID}},{{.Names}},{{.Image}}""#,
        )?;
        let containers = res
            .lines()
            .map(|l| l.split(',').collect_vec())
            .filter(|s| s.len() == 3)
            .map(|s| [s[0], s[1], s[2]])
            .collect_vec();
        let idx = select_idx(
            "",
            &containers.iter().map(|s| s.join(" - ")).collect_vec(),
            "",
        )?;
        let container = containers[idx][0];
        scp_execute(
            &Config::vsdbgsh_path().to_string_lossy(),
            &f!("{host_name}:"),
        )?;
        p!(
            "{}",
            ssh_execute(host_name, &f!("sudo bash vsdbg.sh {container} 4444"))?
        );
        Ok(())
    }
}

fn ssh_execute(host_name: &str, cmd: &str) -> Result<String> {
    Command::new("ssh")
        .args(COMMON_SSH_ARGS)
        .args([host_name, cmd])
        .output()
        .map(|x| String::from_utf8_lossy(&x.stdout).into_owned())
        .map_err(|x| x.into())
}

fn scp_execute(from: &str, to: &str) -> Result<String> {
    Command::new("scp")
        .args(COMMON_SSH_ARGS)
        .args([from, to])
        .output()
        .map(|x| String::from_utf8_lossy(&x.stdout).into_owned())
        .map_err(|x| x.into())
}
