use crate::{describe_instances::DescribeInstances, prelude::*};
use handlebars::{to_json, Handlebars};
use itertools::Itertools;
use serde::Serialize;
use std::{path::Path, process::Command, thread};

#[derive(Serialize)]
struct Instance {
    name: String,
    address: String,
    key: String,
    profile: String,
    platform: String,
    proxy_jump: Option<String>,
}

fn update_from_aws_cli(
    keys_path: impl AsRef<Path>,
    profile: &str,
    proxy_jump: Option<&str>,
) -> Result<Vec<Instance>> {
    let output = Command::new("aws")
        .args([
            "ec2",
            "--profile",
            profile,
            "describe-instances",
            "--filter",
            "Name=instance-state-name,Values=running",
        ])
        .output()?;
    let instances: DescribeInstances = serde_json::from_slice(&output.stdout)?;
    let instances = instances
        .reservations
        .into_iter()
        .flat_map(|x| {
            x.instances.into_iter().filter_map(|i| {
                let name = i.tags.into_iter().find(|x| x.key == "Name")?.value.replace(' ', "-");
                let key = keys_path.as_ref().join(i.key_name?).to_str()?.into();
                let address =
                    if proxy_jump.unwrap_or_default().is_empty() { i.public_ip_address } else { i.private_ip_address };
                Some(Instance {
                    name,
                    key,
                    address,
                    platform: i.platform_details,
                    profile: profile.into(),
                    proxy_jump: proxy_jump.map(String::from),
                })
            })
        })
        .collect_vec();

    Ok(instances)
}

pub fn update_sshconfig(
    profiles: &[String],
    keys_path: impl AsRef<Path>,
    template: impl AsRef<Path>,
    proxy_jump: Option<&str>,
) -> Result<()> {
    let keys_path = keys_path.as_ref();
    let mut srvs: Vec<Instance> = Vec::new();
    thread::scope(|scope| {
        let threads: Vec<_> = profiles
            .iter()
            .map(|profile| {
                scope.spawn(move || {
                    update_from_aws_cli(keys_path, profile, proxy_jump).context(profile.clone())
                })
            })
            .collect();
        for t in threads {
            match &mut t.join() {
                Ok(Ok(add)) => srvs.append(add),
                Ok(Err(err)) => p!("Error updating from cli: {err:#}"),
                Err(_) => p!("A thread panicked"),
            }
        }
    });
    let tmpl = std::fs::read_to_string(template)?;
    let res = Handlebars::new().render_template(&tmpl, &to_json(srvs))?;
    let ssh_config = directories::UserDirs::new()
        .expect("can't retrieve home directory")
        .home_dir()
        .join(".ssh/config");
    std::fs::write(ssh_config, res)?;

    Ok(())
}
