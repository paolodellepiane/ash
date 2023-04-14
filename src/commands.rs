use crate::history::History;
use crate::prelude::*;
use crate::select;
use crate::select::select_teleport_host;
use crate::select::SelectArgs;
use crate::settings::Settings;
use crate::settings::COMMON_TSH_ARGS;
use crate::ssh::Ssh;
use crate::teleport::Host;
use crate::teleport::Hosts;
use clap::arg;
use clap::command;
use clap::Args;
use clap::Subcommand;
use itertools::Itertools;
use std::fs::read;
use std::fs::read_to_string;
use std::fs::DirEntry;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;

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

fn get_hosts(cache_path: impl AsRef<Path>) -> Result<Vec<Host>> {
    let hosts = if cache_path.as_ref().exists() {
        read(&cache_path)?
    } else {
        let hosts = Command::new("tsh").args(COMMON_TSH_ARGS).args(["ls", "-f", "json"]).output()?.stdout;
        std::fs::write(cache_path, &hosts)?;
        hosts
    };
    let hosts: Hosts = serde_json::from_slice(&hosts)?;
    Ok(hosts)
}

fn add_recents(mut hosts: Vec<Host>, history_path: impl AsRef<Path>) -> Vec<Host> {
    let recents = History::load(history_path).intersect(&hosts).entries;
    hosts.retain(|x| !recents.contains(x));
    [recents, hosts].concat()
}

fn select_host(history_path: impl AsRef<Path>, cache_path: impl AsRef<Path>, start_value: &str) -> Result<Host> {
    let hosts = get_hosts(&cache_path)?;
    let hosts = add_recents(hosts, &history_path);
    let host = select_teleport_host(&SelectArgs { hosts, start_value: start_value.into() })?;
    History::load(&history_path).update(&host);
    Ok(host)
}

#[derive(Subcommand)]
pub enum Commands {
    // /// Copy file/folder to/from remote
    // #[command(arg_required_else_help = false, after_help("Folder path not ending with '/' will copy the directory including contents, rather than only the contents of the directory"))]
    // Cp(ScpArgs),
    // /// Create a tunnel for a predefined service
    // #[command(arg_required_else_help = true)]
    // Service {
    //     /// Common Services
    //     service: Service,
    // },
    // /// Create a tunnel for custom ports
    // #[command(arg_required_else_help = true)]
    // Tunnel(TunnelArgs),
    /// Execute a command remotely
    #[command(arg_required_else_help = true)]
    Exec {
        /// Command to execute
        command: String,
    },
    /// Connect vscode to remote host
    #[command()]
    Code,
    /// Get file
    #[command()]
    Get,
    /// Put file
    #[command()]
    Put,
    /// Get windows event logs
    #[command()]
    EventLog,
    /// Append teleport config to ssh config
    #[command()]
    Config,
    // #[command()]
    // Container {
    //     #[command(subcommand)]
    //     container: Container,
    // },
}

#[derive(Subcommand)]
pub enum Container {
    /// Execute a command in a remote container
    #[command(arg_required_else_help = true)]
    Exec {
        /// Command to execute  (e.g. ash container exec "powershell -Command Write-Host $profile")
        #[arg(long_help("e.g.:\n'ash container exec \"powershell -Command Write-Host $profile\"'\n'ash container exec \"cmd /C dir \\\"'"))]
        command: String,
    },
    /// Get file from container
    #[command()]
    Get,
    /// Put file into container
    #[command()]
    Put,
    /// Get windows container event logs
    #[command()]
    EventLog,
    /// Try to setup remote container for remote debug
    #[command()]
    Vsdbg,
}

// pub fn tunnel_from_ports(
//     TunnelArgs { local, remote }: TunnelArgs,
//     hosts @ Hosts { bastion, .. }: &Hosts,
// ) -> Result<()> {
//     if bastion.is_empty() {
//         bail!("Can't tunnel without bastion");
//     }
//     let bastion = hosts.hosts.get(bastion).ok_or_else(|| eyre!("Can't find bastion {bastion:?}"))?.clone();
//     let bastion_name = &bastion.name;
//     let choice = select_profile_then_host(hosts)?;
//     let Host { name, address, .. } = &hosts.hosts[&choice];
//     p!("Tunneling from {local} to {name}:{remote} through {bastion_name} ...");
//     Command::new("ssh")
//         .args(COMMON_SSH_ARGS)
//         .args(["-N", "-L", &f!("{local}:{address}:{remote}"), bastion_name])
//         .status()?;

//     Ok(())
// }

// pub fn tunnel_from_service(service: &Service, hosts: &Hosts) -> Result<()> {
//     let (local, remote) = match service {
//         Service::Rdp => (3389, 3389),
//         Service::Redis => (6379, 6379),
//         Service::Rds => (5432, 5432),
//         Service::RabbitMq => (5672, 5672),
//     };
//     Self::tunnel_from_ports(TunnelArgs { local, remote }, hosts)
// }

// pub fn cp(ScpArgs { from, to }: &ScpArgs, hosts: &Hosts) -> Result<()> {
//     fn expand_remote(s: &str, hosts: &Hosts, is_from: bool) -> Result<String> {
//         if let Some((start_value, path)) = s.rsplit_once(':') {
//             if is_from && path.is_empty() {
//                 bail!("FROM must contain a path to file or folder")
//             }
//             let hosts =
//                 &Hosts { start_value: start_value.to_string(), hosts: hosts.hosts.clone(), bastion: String::new() };
//             let name = select_profile_then_host(hosts)?;
//             let res = f!("{name}:{path}");
//             Ok(res)
//         } else {
//             Ok(String::from(s))
//         }
//     }
//     let mut to = to.to_owned().unwrap_or_default();
//     if to.is_empty() {
//         to = if from.contains(':') { "." } else { ":" }.to_owned() // want to copy from remote to local else from local to remote
//     }
//     if from.contains(':') && to.contains(':') {
//         bail!("Both 'From' and 'To' contain ':'. Use ':' for remote host only")
//     }
//     if !from.contains(':') && !to.contains(':') {
//         bail!("Either 'From' or 'To' must contain ':'. Use ':' for remote host only")
//     }
//     let from = expand_remote(from, hosts, true)?;
//     let to = expand_remote(&to, hosts, false)?;
//     p!("Copying from {from} to {to}...");
//     Command::new("scp").args(COMMON_SSH_ARGS).args(["-r", &from, &to]).status()?;
//     Ok(())
// }

pub fn ssh(s: &Settings) -> Result<()> {
    let host = select_host(&s.history_path, &s.cache_path, &s.start_value)?;
    let name = host.name();
    Command::new("tsh").args(COMMON_TSH_ARGS).args(["ssh", &f!("ubuntu@{name}")]).status()?;
    Ok(())
}

pub fn exec(s: &Settings, command: &str) -> Result<()> {
    let host = select_host(&s.history_path, &s.cache_path, &s.start_value)?;
    let name = host.name();
    Command::new("tsh").args(COMMON_TSH_ARGS).args(["ssh", &f!("ubuntu@{name}"), command]).status()?;
    Ok(())
}

pub fn code(s: &Settings) -> Result<()> {
    append_tsh_to_ssh_config()?;
    let name = &select_host(&s.history_path, &s.cache_path, &s.start_value)?.ssh_name();
    Command::new(&s.code_cmd).args(["--folder-uri", &f!("vscode-remote://ssh-remote+ubuntu@{name}/")]).status()?;
    Ok(())
}

// pub fn win_event_log(hosts: &Hosts) -> Result<()> {
//     let host_name = &select_profile_then_host(hosts)?;
//     if hosts.hosts[host_name].platform != Platform::Win {
//         bail!("This command works for Windows only");
//     }
//     ssh_execute_redirect(
//         host_name,
//         r#"cmd /C "del /Q *.evtx & wevtutil epl System sys.evtx & wevtutil epl Application app.evtx & tar -acf evtx.zip *.evtx""#,
//     )?;
//     scp_execute(&f!("{host_name}:evtx.zip"), ".")?;
//     Ok(())
// }

pub fn append_tsh_to_ssh_config() -> Result<()> {
    let ssh_config =
        directories::UserDirs::new().context("can't retrieve home directory")?.home_dir().join(".ssh").join("config");
    if read_to_string(&ssh_config)?.contains("# Begin generated Teleport configuration") {
        return Ok(());
    }
    let config = Command::new("tsh").args(COMMON_TSH_ARGS).args(["config"]).output()?.stdout;
    let mut f = std::fs::OpenOptions::new().write(true).append(true).open(&ssh_config)?;
    f.write_all(&config)?;
    Ok(())
}

pub fn get_file(s: &Settings) -> Result<()> {
    append_tsh_to_ssh_config()?;
    let host = select_host(&s.history_path, &s.cache_path, &s.start_value)?;
    let path = browse_remote(&host)?;
    scp_execute(&path, ".")?;
    Ok(())
}

pub fn put_file(s: &Settings) -> Result<()> {
    append_tsh_to_ssh_config()?;
    let path = browse_local(s)?;
    let host = select_host(&s.history_path, &s.cache_path, &s.start_value)?;
    let name = host.name();
    scp_execute(&path, &f!("ubuntu@{name}:"))?;
    Ok(())
}

fn browse_local(s: &Settings) -> Result<String> {
    let mut base_dir = s.home_dir.clone();
    loop {
        let entries = read_dir(&base_dir)?;
        let options = entries.iter().map(|x| x.file_name.clone()).filter(|x| x != "./").collect_vec();
        let file = select::select_str("", &options, "")?;
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

fn browse_remote(host: &Host) -> Result<String> {
    let host_name = f!("ubuntu@{}.aws", &host.name());
    let mut ssh = Ssh::new(&host_name)?;
    ssh.write("pwd")?;
    let mut base_dir = ssh.read()?;
    loop {
        ssh.write(&f!("ls --group-directories-first -pa1 {base_dir}"))?;
        let out = ssh.read()?;
        let entries = parse_ls_output(&out, &base_dir)?;
        let options = entries.iter().map(|x| x.file_name.clone()).filter(|x| x != "./").collect_vec();
        let file = select::select_str("", &options, "")?;
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

// fn browse_remote_container(hosts: &Hosts) -> Result<(String, String, String)> {
//     let host_name = &select_profile_then_host(hosts)?;
//     let container = select_container(&hosts.hosts[host_name])?;
//     let mut ssh = Ssh::new(host_name)?;
//     ssh.with_prefix(&f!("sudo docker exec {container}"));
//     ssh.write("pwd")?;
//     let mut base_dir = ssh.read()?;
//     loop {
//         ssh.write(&f!("ls --group-directories-first -pa1 '{base_dir}'"))?;
//         let out = ssh.read()?;
//         let entries = parse_ls_output(&out, &"/")?;
//         let options = entries.iter().map(|x| x.file_name.clone()).filter(|x| x != "./").collect_vec();
//         let file = select_host("", &options, "")?;
//         let entry = entries.iter().find(|x| x.file_name == file).unwrap().clone();
//         if entry.is_dir {
//             if entry.file_name == "../" {
//                 if let Some(parent) = Path::new(&base_dir).parent() {
//                     base_dir = parent.to_string_lossy().into_owned();
//                 }
//             } else {
//                 base_dir = f!("{base_dir}/{}", entry.file_name)
//             }
//         } else {
//             return Ok((host_name.to_string(), dbg!(f!("{container}:{base_dir}/{file}")), file));
//         }
//     }
// }

// impl Container {
//     pub fn win_container_event_log(hosts: &Hosts) -> Result<()> {
//         let host_name = &select_profile_then_host(hosts)?;
//         if hosts.hosts[host_name].platform != Platform::Win {
//             bail!("This command works on Windows only");
//         }
//         let container = select_container(&hosts.hosts[host_name])?;
//         ssh_execute_redirect(
//             host_name,
//             &f!(
//                 r#"docker exec {container} cmd /C "del /Q \*.evtx & wevtutil epl System \sys.evtx & wevtutil epl Application \app.evtx & tar -acf \evtx.zip \*.evtx" && docker cp {container}:\evtx.zip ."#
//             ),
//         )?;
//         scp_execute(&f!("{host_name}:evtx.zip"), ".")?;
//         Ok(())
//     }

//     pub fn vsdbg(hosts: &Hosts) -> Result<()> {
//         let host_name = &select_profile_then_host(hosts)?;
//         let container = select_container(&hosts.hosts[host_name])?;
//         scp_execute(&Config::vsdbgsh_path().to_string_lossy(), &f!("{host_name}:"))?;
//         ssh_execute_redirect(host_name, &f!("sudo bash vsdbg.sh {container} 4444"))?;
//         Ok(())
//     }

//     pub fn exec(command: &str, hosts: &Hosts) -> Result<()> {
//         let host_name = &select_profile_then_host(hosts)?;
//         let container = select_container(&hosts.hosts[host_name])?;
//         ssh_execute_redirect(host_name, &f!(r#"docker exec {container} {command}"#))?;
//         Ok(())
//     }

//     pub fn get_file(hosts: &Hosts) -> Result<()> {
//         let (host_name, path, file) = Commands::browse_remote_container(hosts)?;
//         ssh_execute_redirect(&host_name, &f!(r#"sudo docker cp {path} {file}"#))?;
//         scp_execute(&f!("{host_name}:{file}"), ".")?;
//         Ok(())
//     }

//     pub fn put_file(hosts: &Hosts) -> Result<()> {
//         let path = Commands::browse_local()?;
//         let host_name = &select_profile_then_host(hosts)?;
//         scp_execute(&path, &f!("{host_name}:"))?;
//         Ok(())
//     }
// }

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

// fn select_container(host: &Host) -> Result<String> {
//     let sudo = if host.platform == Platform::Lnx { "sudo " } else { "" };
//     let res = ssh_execute_output(
//         &host.name,
//         &f!(r#"{sudo}docker ps --format "{{{{.ID}}}},{{{{.Names}}}},{{{{.Image}}}}""#),
//     )?;
//     let containers = res
//         .lines()
//         .map(|l| l.split(',').collect_vec())
//         .filter(|s| s.len() == 3)
//         .map(|s| [s[0], s[1], s[2]])
//         .collect_vec();
//     let idx = select_idx("", &containers.iter().map(|s| s.join(" - ")).collect_vec(), "")?;
//     Ok(containers[idx][0].to_string())
// }

// fn ssh_execute_output(host_name: &str, cmd: &str) -> Result<String> {
//     let out = Command::new("tsh").args(COMMON_TSH_ARGS).args([host_name, cmd]).output()?;
//     if !out.status.success() {
//         bail!("{}", String::from_utf8_lossy(&out.stderr));
//     }
//     Ok(String::from_utf8_lossy(&out.stdout).into_owned())
// }

// fn ssh_execute_redirect(host_name: &str, cmd: &str) -> Result<String> {
//     let mut output = Command::new("tsh").args(COMMON_TSH_ARGS).args([host_name, cmd]).stdout(Stdio::piped()).spawn()?;
//     if let Some(stdout) = output.stdout.take() {
//         let out = BufReader::new(stdout).lines().filter_map(|l| l.ok()).inspect(|l| p!("{l}")).collect_vec();
//         return Ok(out.join("\n"));
//     }
//     Ok(String::from(""))
// }

fn scp_execute(from: &str, to: &str) -> std::io::Result<ExitStatus> {
    Command::new("scp").args(["-T"]).args([from, to]).status()
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
