#![warn(clippy::all)]
use crate::options::COMMON_TSH_ARGS;
use history::History;
use options::Options;
use prelude::*;
use select::{select_teleport_host, SelectArgs};
use std::{path::Path, process::Command};
use teleport::{Host, Hosts};

mod history;
mod options;
mod prelude;
mod select;
mod teleport;

pub fn ssh(host: &Host) -> Result<()> {
    let name = &host.spec.hostname;
    p!("Connecting to {name}...");
    Command::new("tsh").args(COMMON_TSH_ARGS).args(["ssh", &f!("ubuntu@{name}")]).status()?;
    Ok(())
}

fn get_hosts() -> Result<Vec<Host>> {
    let hosts = Command::new("tsh").args(COMMON_TSH_ARGS).args(["ls", "-f", "json"]).output()?.stdout;
    let hosts: Hosts = serde_json::from_slice(&hosts)?;
    Ok(hosts)
}

fn add_recents(mut hosts: Vec<Host>, history_path: impl AsRef<Path>) -> Vec<Host> {
    let recents = History::load(history_path).intersect(&hosts).entries;
    hosts.retain(|x| !recents.contains(x));
    [recents, hosts].concat()
}

fn main() -> Result<()> {
    let opt = Options::new()?;
    let hosts = get_hosts()?;
    let hosts = add_recents(hosts, &opt.history_path);
    let start_value = opt.args.host.unwrap_or_default();
    let host = select_teleport_host(&SelectArgs { hosts, start_value })?;
    History::load(&opt.history_path).update(&host);
    ssh(&host)?;
    Ok(())
}
