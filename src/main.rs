#![warn(clippy::all)]
use crate::select::History;
use aws::update_sshconfig;
use commands::*;
use config::{Config, CFG};
use parsers::ssh_config_parser::parse_ssh_config_from_host;
use prelude::*;
use std::process::exit;

mod aws;
mod commands;
mod config;
mod describe_instances;
mod parsers;
mod prelude;
mod select;
mod ssh;

fn run() -> Result<()> {
    let (config, args) = &*CFG;
    if args.check_update {
        if cfg!(windows) {
            std::process::Command::new("scoop.cmd").args(["update", "-k", "ash"]).spawn()?;
        } else {
            p!("not implemented on this platform");
        }
        exit(0);
    }
    if config.update {
        update_sshconfig(
            &config.keys_path,
            Config::template_path(),
            &config.bastion_name,
        )?;
    }
    let hosts = parse_ssh_config_from_host()?;
    History::intersect(&hosts);
    let hosts = &Hosts {
        hosts,
        start_value: args.host.clone().unwrap_or_default(),
        bastion: config.bastion_name.clone(),
    };
    use Commands::*;
    match &args.command {
        Some(cmd) => match cmd {
            Cp(cp) => Commands::cp(cp, hosts),
            Service { service } => Commands::tunnel_from_service(service, hosts),
            Tunnel(tunnel) => Commands::tunnel_from_ports(*tunnel, hosts),
            Exec { command } => Commands::exec(command, hosts),
            Code => Commands::code(hosts),
            Info => Commands::info(hosts),
            Vsdbg => Commands::vsdbg(hosts),
            EventLog => Commands::win_event_log(hosts),
            ContainerEventLog => Commands::win_container_event_log(hosts),
            Get => Commands::get_file(hosts),
            Put => Commands::put_file(hosts),
        },
        None => Commands::ssh(hosts),
    }
}

fn main() -> Result<()> {
    run()
}
