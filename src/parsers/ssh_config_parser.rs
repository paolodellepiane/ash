use crate::{config::Config, prelude::*};
use pest::Parser;
use pest_derive::Parser;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum Platform {
    Win,
    Lnx,
}

#[derive(Clone, Debug, Serialize)]
pub struct Host {
    pub name: String,
    pub profile: String,
    pub address: String,
    pub platform: Platform,
    pub user: Option<String>,
    pub key: Option<String>,
    pub bastion: Option<String>,
}

#[derive(Parser)]
#[grammar = "parsers/pegs/sshconfig.pest"]
pub struct SshConfigParser;

// https://www.ssh.com/academy/ssh/config
pub fn parse_ssh_config(content: &str) -> Result<HashMap<String, Host>> {
<<<<<<< HEAD
=======
    // stopwatch!();
>>>>>>> fdae2ab (tmp)
    let res = SshConfigParser::parse(Rule::file, content)?.next().unwrap();
    let mut hosts: HashMap<&str, HashMap<String, &str>> = HashMap::new();
    let mut current_host = "";
    for line in res.into_inner() {
        match line.as_rule() {
            Rule::host => {
                current_host = line.into_inner().next().unwrap().as_str();
            }
            Rule::profile => {
                let description = line.into_inner().next().unwrap().as_str();
                let (profile, platform) = description
                    .split_once(',')
                    .ok_or_else(|| eyre!("can't get profile and platform from '{description}'"))?;
                hosts
                    .entry(current_host)
                    .or_default()
                    .insert("profile".to_string(), profile.trim());
                hosts
                    .entry(current_host)
                    .or_default()
                    .insert("platform".to_string(), platform.trim());
            }
            Rule::option => {
                let rules = &mut line.into_inner();
                let keyword = rules.next().unwrap().as_str();
                let argument = rules.next().unwrap().as_str();
                hosts.entry(current_host).or_default().insert(keyword.to_lowercase(), argument);
            }
            _ => (),
        }
    }
    let res: HashMap<_, _> = hosts
        .into_iter()
        .filter_map(|(name, o)| {
            let name = name.to_string();
            let profile = o.get("profile").copied().unwrap_or("others").to_string();
            let platform = o.get("platform").copied().unwrap_or("others").to_string();
            let platform = if platform == "win" { Platform::Win } else { Platform::Lnx };
            let address = o.get("hostname")?.to_string();
            let user = o.get("user").copied().map(String::from);
            let key = o.get("identityfile").copied().map(String::from);
            let bastion = o.get("proxyjump").copied().map(String::from);
            Some((
                name.clone(),
                Host { name, profile, address, user, key, bastion, platform },
            ))
        })
        .collect();
    Ok(res)
}

pub fn parse_ssh_config_from_host() -> Result<HashMap<String, Host>> {
    let path = Config::home_dir().join(".ssh").join("config");
    let ssh_config = std::fs::read_to_string(&path).context(f!("can't read {path:?}"))?;
    parse_ssh_config(&ssh_config)
}

#[cfg(test)]
mod tests {
    const SSH_CONFIG: &str = r#"
Host *
StrictHostKeyChecking no

Host mybastionexample
    HostName 3.248.182.201
    User ubuntu

Host audit
# profile prod
    HostName 52.208.85.57
    User ubuntu
    IdentityFile /Users/paolo/.ssh/m4cprod-key
    
           # test
        
Host RRM-01-(mago)
# profile prod
    HostName 108.128.124.210

    User ubuntu
    # aaa
    IdentityFile /Users/paolo/.ssh/m4cprod-key    
"#;

    #[test]
    fn parse_ssh_config_succeeds() {
        let res = super::parse_ssh_config(SSH_CONFIG);
        match res {
            Ok(r) => assert_eq!(r.len(), 3),
            Err(err) => panic!("{err:#}"),
        }
    }
}
