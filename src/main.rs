#![warn(clippy::all)]
use crate::{config::COMMON_TSH_ARGS, select::History};
use clap::Parser;
use config::AshArgs;
use prelude::*;
use select::{select_teleport_host, SelectArgs};
use std::process::Command;
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

fn check_ash_update() -> Result<()> {
    if cfg!(windows) {
        std::process::Command::new("scoop.cmd").args(["update", "-k", "ash"]).spawn()?;
    } else {
        bail!("not implemented on this platform");
    }
    Ok(())
}

fn run() -> Result<()> {
    let args = AshArgs::parse();
    if args.check_update {
        return check_ash_update();
    }
    let start_value = args.host.unwrap_or_default();
    let hosts = Command::new("tsh").args(COMMON_TSH_ARGS).args(["ls", "-f", "json"]).output()?.stdout;
    let hosts: Welcome = serde_json::from_slice(&hosts)?;
    let host = select_teleport_host(SelectArgs { hosts, start_value })?;
    ssh(&host)?;
    Ok(())
}

fn main() -> Result<()> {
    run()
}
