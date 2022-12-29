#![warn(clippy::all)]
use aws::update_sshconfig;
use commands::*;
use config::{Config, CFG};
use dioxus::prelude::*;
use dioxus_elements::input_data::keyboard_types::Key;
use itertools::Itertools;
use parsers::ssh_config_parser::parse_ssh_config_from_host;
use prelude::*;
use std::iter::once;
use std::process::exit;

mod aws;
mod commands;
mod config;
mod describe_instances;
mod parsers;
mod prelude;
mod ssh;

fn select(message: &str, options: &Vec<String>, start_value: &str) -> Result<String> {
    let message = message.to_owned();
    let options = options.iter().map(|x| x.to_owned()).collect_vec();
    launch(app, SelectProps { message, options }).ok_or_else(|| eyre!("no host selected"))
}

fn select_profile_then_host(Hosts { hosts, start_value, .. }: &Hosts) -> Result<String> {
    if CFG.0.merge_profiles {
        let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
        return select("", &values, start_value);
    }
    let _select_profile_then_host = |(start_profile, start_host): (&str, &str)| {
        let profiles = hosts.iter().map(|(_, h)| h.profile.clone()).unique();
        let profiles = once("history".to_string()).chain(profiles).collect_vec();
        let profile = select("", &profiles, start_profile)?;
        let values = hosts
            .iter()
            .filter_map(|(_, h)| (h.profile == profile).then_some(h.name.clone()))
            .collect_vec();
        if profile == "history" {
            select(&f!("[{profile}]"), &values, start_host)
        } else {
            select(&f!("[{profile}]"), &values, start_host)
        }
    };
    match start_value {
        sv if sv.contains(':') => _select_profile_then_host(sv.split_once(':').unwrap()),
        sv if sv.is_empty() => _select_profile_then_host(("", "")),
        _ => {
            let values = hosts.iter().map(|(name, _)| name.clone()).collect_vec();
            select("", &values, start_value)
        }
    }
}

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

struct SelectProps {
    message: String,
    options: Vec<String>,
}

fn app(cx: Scope<WithResult<SelectProps, String>>) -> Element {
    let selected = use_state(cx, || 2_usize);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",
            border_width: "1px",
            onkeydown: move |event| {
                p!("{event:?}");
                match event.key() {
                    Key::ArrowDown => selected.set(1),
                    Key::ArrowUp => selected.set(1),
                    _ => {}
                };
            },

            h1 { height: "2px", color: "green",
                "{cx.props.props.message}"
            }

            ul { flex_direction: "column", padding_left: "3px",
                cx.props.props.options.iter().enumerate().map(|(i, o)| {
                    if i == *selected.get() {
                        rsx!(
                            "> {o}"
                        )
                    } else {
                        rsx!("  {o}")
                    }
                })
            }
        }
    })
}
