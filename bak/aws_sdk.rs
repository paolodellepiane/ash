use crate::prelude::*;
use aws_config::default_provider::credentials::DefaultCredentialsChain;
use futures::future::try_join_all;
use handlebars::{to_json, Handlebars};
use itertools::Itertools;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
struct Instance {
    name: String,
    address: String,
    key: String,
    profile: String,
    platform: String,
    proxy_jump: Option<String>,
}

async fn update(keys_path: impl AsRef<Path>, profile: &str, proxy_jump: Option<&str>) -> Result<Vec<Instance>> {
    let creds = DefaultCredentialsChain::builder().profile_name(profile).build().await;
    let aws_cfg = aws_config::from_env().credentials_provider(creds).load().await;
    let client = aws_sdk_ec2::Client::new(&aws_cfg);
    let filter =
        aws_sdk_ec2::model::Filter::builder().name("instance-state-name").values("running").build();
    let resp = client.describe_instances().filters(filter).send().await?;
    let instances = resp
        .reservations
        .unwrap_or_default()
        .into_iter()
        .flat_map(|x| {
            x.instances.unwrap().into_iter().filter_map(|i| {
                let name =
                    i.tags?.into_iter().find(|x| x.key().unwrap_or_default() == "Name")?.value?.replace(' ', "-");
                let key = keys_path.as_ref().join(i.key_name?).to_str()?.into();
                let address = if proxy_jump.is_some() { i.private_ip_address } else { i.public_ip_address }?;
                Some(Instance {
                    name,
                    key,
                    address,
                    platform: i.platform_details?,
                    profile: profile.into(),
                    proxy_jump: proxy_jump.map(String::from),
                })
            })
        })
        .collect_vec();

    Ok(instances)
}

pub async fn update_sshconfig(profiles: &[String], keys_path: impl AsRef<Path>, proxy_jump: Option<&str>) -> Result<()> {
    let srvs_futures = profiles.iter().map(|p| update(keys_path.as_ref(), p, proxy_jump));
    let srvs = try_join_all(srvs_futures).await?.into_iter().flatten().collect_vec();
    let tmpl = std::fs::read_to_string("res/template.for.sshconfig.hbs")?;
    let res = Handlebars::new().render_template(&tmpl, &to_json(srvs))?;
    let ssh_config = directories::UserDirs::new()
        .expect("can't retrieve home directory")
        .home_dir()
        .join(".ssh/config");
    std::fs::write(ssh_config, res)?;

    Ok(())
}
