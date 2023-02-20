#![warn(clippy::all)]
use crate::config::COMMON_TSH_ARGS;
use clap::Parser;
use config::AshArgs;
use history::History;
use prelude::*;
use select::{select_teleport_host, SelectArgs};
use std::process::Command;
use teleport::{Host, Hosts};

mod config;
mod history;
mod prelude;
mod select;
mod ssh;
mod teleport;

pub fn ssh(host: &Host) -> Result<()> {
    let name = &host.spec.hostname;
    p!("Connecting to {name}...");
    Command::new("tsh").args(COMMON_TSH_ARGS).args(["ssh", &f!("ubuntu@{name}")]).status()?;
    Ok(())
}

fn check_ash_update() -> Result<()> {
    if cfg!(windows) {
        std::process::Command::new("scoop.cmd").args(["update", "-k", "ash"]).spawn()?;
    } else {
        bail!("not implemented on this platform");
    }
    Ok(())
}

fn get_hosts() -> Result<Vec<Host>> {
    let hosts = Command::new("tsh").args(COMMON_TSH_ARGS).args(["ls", "-f", "json"]).output()?.stdout;
    let hosts: Hosts = serde_json::from_slice(&hosts)?;
    Ok(hosts)
}

fn add_recents(mut hosts: Vec<Host>) -> Vec<Host> {
    History::intersect(&hosts);
    let recents = History::load().entries;
    hosts.retain(|x| !recents.contains(x));
    [recents, hosts].concat()
}

fn run() -> Result<()> {
    let args = AshArgs::parse();
    if args.check_update {
        return check_ash_update();
    }
    let hosts = get_hosts()?;
    let hosts = add_recents(hosts);
    let start_value = args.host.unwrap_or_default();
    let host = select_teleport_host(&SelectArgs { hosts, start_value })?;
    ssh(&host)?;
    Ok(())
}

fn main() -> Result<()> {
    run()
}
