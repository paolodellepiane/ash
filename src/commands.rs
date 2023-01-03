use crate::config::Config;
use crate::config::Service;
use crate::config::COMMON_SSH_ARGS;
use crate::parsers::ssh_config_parser::Host;
use crate::parsers::ssh_config_parser::Platform;
use crate::prelude::*;
use crate::select::*;
use crate::ssh::Ssh;
use clap::arg;
use clap::command;
use clap::Args;
use clap::Subcommand;
use itertools::Itertools;
use std::collections::HashMap;
use std::fs::DirEntry;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

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
    /// Get windows event logs
    #[command()]
    EventLog,
    /// Get windows container event logs
    #[command()]
    ContainerEventLog,
    /// Get file
    #[command()]
    Get,
    /// Put file
    #[command()]
    Put,
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
        let container = select_container(&hosts.hosts[host_name])?;
        scp_execute(
            &Config::vsdbgsh_path().to_string_lossy(),
            &f!("{host_name}:"),
        )?;
        ssh_execute_redirect(host_name, &f!("sudo bash vsdbg.sh {container} 4444"))?;
        Ok(())
    }

    pub fn win_event_log(hosts: &Hosts) -> Result<()> {
        let host_name = &select_profile_then_host(hosts)?;
        if hosts.hosts[host_name].platform != Platform::Win {
            bail!("This command works for Windows only");
        }
        ssh_execute_redirect(
            host_name,
            r#"cmd /C "del /Q *.evtx & wevtutil epl System sys.evtx & wevtutil epl Application app.evtx & tar -acf evtx.zip *.evtx""#,
        )?;
        scp_execute(&f!("{host_name}:evtx.zip"), ".")?;
        Ok(())
    }

    pub fn win_container_event_log(hosts: &Hosts) -> Result<()> {
        let host_name = &select_profile_then_host(hosts)?;
        if hosts.hosts[host_name].platform != Platform::Win {
            bail!("This command works on Windows only");
        }
        let container = select_container(&hosts.hosts[host_name])?;
        ssh_execute_redirect(
            host_name,
            &f!(
                r#"docker exec {container} cmd /C "del /Q \*.evtx & wevtutil epl System \sys.evtx & wevtutil epl Application \app.evtx & tar -acf \evtx.zip \*.evtx""#
            ),
        )?;
        ssh_execute_redirect(host_name, &f!(r#"docker cp {container}:\evtx.zip .""#))?;
        scp_execute(&f!("{host_name}:evtx.zip"), ".")?;
        Ok(())
    }

    pub fn get_file(hosts: &Hosts) -> Result<()> {
        let path = Self::browse_remote(hosts)?;
        scp_execute(&path, ".")?;
        Ok(())
    }

    pub fn put_file(hosts: &Hosts) -> Result<()> {
        let path = Self::browse_local()?;
        let host_name = &select_profile_then_host(hosts)?;
        scp_execute(&path, &f!("{host_name}:"))?;
        Ok(())
    }

    fn browse_local() -> Result<String> {
        let mut base_dir = Config::home_dir();
        loop {
            let entries = read_dir(&base_dir)?;
            let options =
                entries.iter().map(|x| x.file_name.clone()).filter(|x| x != "./").collect_vec();
            let file = select("", &options, "")?;
            let entry = entries.iter().find(|x| x.file_name == file).unwrap().clone();
            if entry.is_dir {
                if entry.file_name == "../" {
                    if let Some(parent) = Path::new(&base_dir).parent() {
                        base_dir = parent.to_owned();
                    }
                } else {
                    base_dir = base_dir.join(entry.file_name);
                }
            } else {
                return Ok(base_dir.join(file).to_string_lossy().into_owned());
            }
        }
    }

    fn browse_remote(hosts: &Hosts) -> Result<String> {
        let host_name = &select_profile_then_host(hosts)?;
        let mut ssh = Ssh::new(host_name)?;
        ssh.write("pwd")?;
        let mut base_dir = ssh.read()?;
        loop {
            ssh.write(&f!("ls --group-directories-first -pa1 '{base_dir}'"))?;
            let out = ssh.read()?;
            let entries = parse_ls_output(&out, &"/")?;
            let options =
                entries.iter().map(|x| x.file_name.clone()).filter(|x| x != "./").collect_vec();
            let file = select("", &options, "")?;
            let entry = entries.iter().find(|x| x.file_name == file).unwrap().clone();
            if entry.is_dir {
                if entry.file_name == "../" {
                    if let Some(parent) = Path::new(&base_dir).parent() {
                        base_dir = parent.to_string_lossy().into_owned();
                    }
                } else {
                    base_dir = f!("{base_dir}/{}", entry.file_name)
                }
            } else {
                return Ok(f!("{host_name}:{base_dir}/{file}"));
            }
        }
    }
}

pub fn read_dir(path: impl AsRef<Path>) -> Result<Vec<Entry>> {
    let files = std::fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(Entry::from)
        .sorted_by_key(|x| {
            let p = if x.is_dir { "a" } else { "b" };
            f!("{p}{}", x.file_name)
        })
        .collect();
    Ok(files)
}

fn select_container(host: &Host) -> Result<String> {
    let sudo = if host.platform == Platform::Lnx { "sudo " } else { "" };
    let res = ssh_execute(
        &host.name,
        &f!(r#"{sudo}docker ps --format "{{{{.ID}}}},{{{{.Names}}}},{{{{.Image}}}}""#),
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
    Ok(containers[idx][0].to_string())
}

fn ssh_execute(host_name: &str, cmd: &str) -> Result<String> {
    let out = Command::new("ssh").args(COMMON_SSH_ARGS).args([host_name, cmd]).output()?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn ssh_execute_redirect(host_name: &str, cmd: &str) -> Result<String> {
    let mut output = Command::new("ssh")
        .args(COMMON_SSH_ARGS)
        .args([host_name, cmd])
        .stdout(Stdio::piped())
        .spawn()?;
    if let Some(stdout) = output.stdout.take() {
        let out = BufReader::new(stdout)
            .lines()
            .filter_map(|l| l.ok())
            .inspect(|l| p!("{l}"))
            .collect_vec();
        return Ok(out.join("\n"));
    }
    Ok(String::from(""))
}

fn scp_execute(from: &str, to: &str) -> Result<String> {
    let out = Command::new("scp").args(COMMON_SSH_ARGS).args([from, to]).output()?.stdout;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub file_name: String,
    pub is_dir: bool,
    pub is_selected: bool,
}

impl From<DirEntry> for Entry {
    fn from(e: DirEntry) -> Self {
        Self {
            path: e.path(),
            file_name: e.file_name().to_string_lossy().to_string(),
            is_dir: e.path().is_dir(),
            is_selected: false,
        }
    }
}

fn parse_ls_output(ls_output: &str, base_path: &impl AsRef<Path>) -> Result<Vec<Entry>> {
    let res = ls_output
        .lines()
        .map(|x| Entry {
            file_name: x.into(),
            path: base_path.as_ref().join(x),
            is_dir: x.ends_with('/'),
            is_selected: false,
        })
        .sorted_by_key(|x| if x.is_dir { "a" } else { "b" })
        .collect();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ls_output_succeeds() {
        const LS: &str = r#"
./
../
.DS_Store
.git/
.gitignore
.vscode/
Cargo.lock
Cargo.toml
ash
ash.config.json
clippy.sh
res/
rustfmt.toml
src/
target/
test.txt
"#;

        let res = parse_ls_output(LS, &"/test/");
        assert!(res.is_ok());
        println!("{:#?}", res.unwrap());
    }
}
