use crate::config::Config;
use crate::config::TunnelConfig;
use crate::config::COMMON_SSH_ARGS;
use crate::parsers::ssh_config_parser::Host;
use crate::prelude::*;
use itertools::Itertools;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc::channel;
use std::thread::spawn;

pub trait Executable {
    fn exec(&self) -> Result<()>;
}

#[derive(Default, Clone)]
pub struct ExecOpt {
    pub hosts: HashMap<String, Host>,
    pub host: String,
    pub start_value: String,
}

#[derive(Clone)]
pub struct Tunnel {
    cfg: TunnelConfig,
    host: Host,
}

impl Tunnel {
    pub fn new(name: &str, opt: &ExecOpt, cfg: &Config) -> Result<Self> {
        ensure!(!opt.host.is_empty(), "Host can't be empty");
        ensure!(!name.is_empty(), "Tunnel name can't be empty");
        let tuns = &cfg.tunnels;
        let cfg = tuns
            .get(name)
            .ok_or_else(|| eyre!("Valid services are: {}", tuns.keys().join(", ")))?
            .clone();
        let host = opt.hosts[&opt.host].clone();
        if host.bastion.is_none() {
            bail!("Host {} has no bastion configured", opt.host);
        }
        Ok(Self { cfg, host })
    }
}

fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn get_ephemeral_port() -> Result<u16> {
    stopwatch!();
    (32768..60999)
        .find(|&x| port_is_available(x))
        .ok_or_else(|| eyre!("Can't get an ephemeral port"))
}

impl Executable for Tunnel {
    fn exec(&self) -> Result<()> {
        let Self {
            cfg: TunnelConfig { local, remote, run },
            host: Host { name, address, bastion, user, .. },
        } = self;
        let local = if let Some(local) = *local { local } else { get_ephemeral_port()? };
        let (tx, rx) = channel();
        if !run.is_empty() {
            let user = user.clone().unwrap_or_default();
            let args = run[1..]
                .iter()
                .map(|s| s.replace("${port}", &local.to_string()).replace("${user}", &user))
                .collect_vec();
            spawn({
                let cmd = run[0].clone();
                move || {
                    rx.recv().expect("Could not receive ready signal from main thread.");
                    if let Err(err) = Command::new(&cmd).args(args).status() {
                        p!("Can't run {cmd}: {err:?}");
                    };
                }
            });
        }
        p!("Tunneling from {local} to {name}:{remote} through {bastion:?} ...");
        let mut child = Command::new("ssh")
            .args(COMMON_SSH_ARGS)
            .args([
                "-v",
                "-N",
                "-L",
                &f!("{local}:{address}:{remote}"),
                bastion.as_ref().unwrap(),
            ])
            .stderr(Stdio::piped())
            .spawn()?;
        let reader = BufReader::new(child.stderr.as_mut().unwrap());
        reader.lines().filter_map(|line| line.ok()).for_each(|line| {
            p!("{line}");
            if line.contains("Entering interactive session.") {
                tx.send(()).expect("Could not send redy signal to rdp thread.");
            }
        });
        Ok(())
    }
}

// #[derive(Args)]
// pub struct ScpArgs {
//     /// From    (use ':' to copy from remote, e.g. 'ash cp <remote>:fake.toml .')
//     #[arg(long_help("use ':' to copy from remote, e.g.:\n'ash cp <remote>:fake.toml .' : copy fake:toml from <remote> to current dir\n<remote> can be empty or partial, ash will ask to select it from a list"))]
//     pub from: String,
//     /// To    (use ':' to copy to remote, e.g. 'ash cp fake.toml <remote>:fake.toml')
//     #[arg(long_help("use ':' to copy to remote, e.g.:\n'ash cp fake.toml <remote>:fake.toml .' : copy fake:toml from current dir to <remote>\n<remote> can be empty or partial, ash will ask to select it from a list"))]
//     pub to: String,
// }

// #[derive(Clone)]
// pub struct Scp {
//     from: String,
//     to: String,
// }

// impl Scp {
//     pub fn new(ScpArgs { from, to }: &ScpArgs, hosts: &Hosts) -> Result<Self> {
//         fn expand_remote(s: &str, hosts: &Hosts) -> Result<(String, Option<Host>)> {
//             if let Some((start_value, path)) = s.rsplit_once(':') {
//                 let hosts = &Hosts {
//                     start_value: start_value.to_string(),
//                     hosts: hosts.hosts.clone(),
//                     bastion: String::new(),
//                 };
//                 let choice = select_profile_then_host(hosts)?;
//                 let host @ Host { name, .. } = &hosts.hosts[&choice];
//                 let res = f!("{name}:{path}");
//                 Ok((res, Some(host.clone())))
//             } else {
//                 Ok((String::from(s), None))
//             }
//         }
//         if from.contains(':') && to.contains(':') {
//             bail!("Both 'From' and 'To' contain ':'. Use ':' for remote host only")
//         }
//         if !from.contains(':') && !to.contains(':') {
//             bail!("Either 'From' or 'To' must contain ':'. Use ':' for remote host only")
//         }
//         let (from, from_host) = expand_remote(from, hosts)?;
//         let (to, to_host) = expand_remote(to, hosts)?;
//         from_host.or(to_host).context("No host found")?;
//         Ok(Self { from, to })
//     }
// }

// impl Executable for Scp {
//     fn exec(&self) -> Result<()> {
//         let Self { from, to, .. } = self;
//         p!("Copying from {from} to {to}...");
//         Command::new("scp").args(COMMON_SSH_ARGS).args([from, to]).status()?;
//         Ok(())
//     }
// }

pub struct Ssh {
    host: Host,
}

impl Ssh {
    pub fn new(opt: &ExecOpt) -> Result<Self> {
        ensure!(!opt.host.is_empty(), "Host can't be empty");
        Ok(Self { host: opt.hosts[&opt.host].clone() })
    }
}

impl Executable for Ssh {
    fn exec(&self) -> Result<()> {
        let Host { name, .. } = &self.host;
        let cmd = COMMON_SSH_ARGS.join(" ") + " " + name;
        p!("Executing {cmd}...");
        Command::new("osascript")
            .args([
                "-e",
                &f!("tell app \"Terminal\" to do script \"ssh {cmd}\""),
            ])
            .spawn()?;
        Ok(())
    }
}

// pub struct Exec {
//     host: Host,
//     command: String,
// }

// impl Exec {
//     pub fn new(command: &str, hosts: &Hosts) -> Result<Self> {
//         let choice = select_profile_then_host(hosts)?;
//         Ok(Self { host: hosts.hosts[&choice].clone(), command: command.to_string() })
//     }
// }

// impl Executable for Exec {
//     fn exec(&self) -> Result<()> {
//         let Self { command, host: Host { name, .. } } = self;
//         p!("Executing on {name}...");
//         Command::new("ssh").args(COMMON_SSH_ARGS).args([name, command]).status()?;
//         Ok(())
//     }
// }

pub struct Code {
    host: Host,
}

impl Code {
    pub fn new(opt: &ExecOpt) -> Result<Self> {
        // let choice = select_profile_then_host(opt)?;
        ensure!(!opt.host.is_empty(), "Host can't be empty");
        Ok(Self { host: opt.hosts[&opt.host].clone() })
    }
}

impl Executable for Code {
    fn exec(&self) -> Result<()> {
        let Self { host: Host { name, .. } } = self;
        p!("Connect vscode to remote host {name}...");
        Command::new("code")
            .args(["--folder-uri", &f!("vscode-remote://ssh-remote+{name}/")])
            .status()?;
        Ok(())
    }
}
