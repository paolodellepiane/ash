#![warn(clippy::all)]
use crate::{config::COMMON_TSH_ARGS, select::History};
use config::CFG;
use prelude::*;
use select::{select_teleport_host, SelectArgs};
use std::process::{exit, Command};
use teleport::{Welcome, WelcomeElement};

mod config;
mod prelude;
mod select;
mod ssh;
mod teleport;

pub fn ssh(host: &WelcomeElement) -> Result<()> {
    let name = &host.spec.hostname;
    p!("Connecting to {name}...");
    Command::new("tsh").args(COMMON_TSH_ARGS).args(["ssh", &f!("ubuntu@{name}")]).status()?;
    Ok(())
}

fn _requires_login() -> Result<bool> {
    let status = Command::new("tsh").args(["status"]).output()?;
    Ok(std::str::from_utf8(&status.stderr).unwrap().contains("Not logged in"))
}

fn run() -> Result<()> {
    let (_, args) = &*CFG;
    if args.check_update {
        if cfg!(windows) {
            std::process::Command::new("scoop.cmd").args(["update", "-k", "ash"]).spawn()?;
        } else {
            p!("not implemented on this platform");
        }
        return Ok(());
    }
    let start_value = args.host.clone().unwrap_or_default();
    let hosts =
        Command::new("tsh").args(COMMON_TSH_ARGS).args(["ls", "-f", "json"]).output()?.stdout;
    let hosts: Welcome = serde_json::from_slice(&hosts)?;
    History::intersect(&hosts);
    let host = select_teleport_host(&SelectArgs { hosts, start_value })?;
    ssh(&host)?;
    Ok(())
}

fn main() -> Result<()> {
    run()
}
