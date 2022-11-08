use crate::config::Config;
use crate::prelude::*;
use aws_sigv4::http_request::{sign, SignableRequest, SigningParams, SigningSettings};
use handlebars::{to_json, Handlebars};
use http::request::Parts;
use http::Request;
use ini::Ini;
use itertools::Itertools;
use minreq::{Response, URL};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::var;
use std::fs::File;
use std::time::SystemTime;
use std::{path::Path, str::FromStr, thread};

#[derive(Serialize)]
struct Instance {
    name: String,
    address: String,
    key: String,
    profile: String,
    platform: String,
    proxy_jump: Option<String>,
    user: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
struct Credential {
    profile: String,
    access_key: String,
    secret: String,
    token: String,
    region: String,
    expiration: String,
}

trait NodeExt {
    fn find_tag(&self, name: &str) -> Option<Node>;
}

impl<'a, 'input> NodeExt for Node<'a, 'input> {
    fn find_tag(&self, name: &str) -> Option<Node> {
        self.children().find(|t| t.has_tag_name(name))
    }
}

impl<'input> NodeExt for Document<'input> {
    fn find_tag(&self, name: &str) -> Option<Node> {
        self.descendants().find(|t| t.has_tag_name(name))
    }
}

impl Credential {
    fn is_expired(&self) -> bool {
        let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&self.expiration) else {
            return false;
        };
        exp < chrono::Utc::now()
    }
}

#[derive(Debug)]
struct AwsConfig {
    profile: String,
    role_arn: String,
    source_profile: String,
    region: String,
}

fn aws_sign(
    req: &mut Request<impl AsRef<[u8]>>,
    service_name: &str,
    Credential { access_key, secret, token, region, .. }: &Credential,
) -> Result<()> {
    let signing_settings = SigningSettings::default();
    let mut signing_params = SigningParams::builder()
        .access_key(access_key)
        .secret_key(secret)
        .region(region)
        .service_name(service_name)
        .time(SystemTime::now())
        .settings(signing_settings);
    if !token.is_empty() {
        signing_params = signing_params.security_token(token)
    }
    let signing_params = signing_params.build()?;
    let signable_request = SignableRequest::from(&*req);
    let (signing_instructions, _signature) =
        sign(signable_request, &signing_params).unwrap().into_parts();
    signing_instructions.apply_to_request(req);
    Ok(())
}

fn aws_send(url: &str, service_name: &str, cred: &Credential) -> Result<Response> {
    let mut req = Request::builder().uri(url).body("").unwrap();
    aws_sign(&mut req, service_name, cred)?;
    let (parts, _) = req.into_parts();
    let Parts { uri, headers, .. } = parts;
    let mut req = minreq::Request::new(minreq::Method::Get, URL::from_str(&uri.to_string())?);
    for (k, v) in headers.iter() {
        req = req.with_header(k.as_str(), v.to_str()?);
    }
    let res = req.send()?;
    Ok(res)
}

fn get_sts_creds(
    AwsConfig { profile, role_arn, region, .. }: AwsConfig,
    cred: &Credential,
) -> Result<Credential> {
    let assume_role = f!("https://sts.{region}.amazonaws.com/?Version=2011-06-15&Action=AssumeRole&RoleSessionName={profile}&RoleArn={role_arn}&DurationSeconds=3600");
    let res = aws_send(&assume_role, "sts", cred)?;
    if res.status_code < 200 || res.status_code > 299 {
        bail!(
            "Error assuming role for {profile}, {}, {}",
            res.status_code,
            res.reason_phrase
        )
    }
    let doc = roxmltree::Document::parse(res.as_str()?)?;
    let access_key = doc
        .find_tag("AccessKeyId")
        .and_then(|x| x.text())
        .ok_or_else(|| anyhow!("can't get access_key for {profile}"))?
        .to_string();
    let secret = doc
        .find_tag("SecretAccessKey")
        .and_then(|x| x.text())
        .ok_or_else(|| anyhow!("can't get secret for {profile}"))?
        .to_string();
    let token = doc
        .find_tag("SessionToken")
        .and_then(|x| x.text())
        .ok_or_else(|| anyhow!("can't get token for {profile}"))?
        .to_string();
    let expiration = doc
        .find_tag("Expiration")
        .and_then(|x| x.text())
        .ok_or_else(|| anyhow!("can't get expiration for {profile}"))?
        .to_string();

    Ok(Credential { access_key, secret, token, profile, region, expiration })
}

fn get_env_credentials() -> Result<Credential> {
    Ok(Credential {
        profile: "default".into(),
        access_key: var("AWS_ACCESS_KEY_ID")?,
        secret: var("AWS_SECRET_ACCESS_KEY")?,
        region: var("AWS_DEFAULT_REGION")?,
        ..Credential::default()
    })
}

fn get_shared_credentials() -> Result<Vec<Credential>> {
    let user_dirs = Config::user_dirs();
    let aws_credentials = user_dirs.home_dir().join(".aws").join("credentials");
    let aws_config = user_dirs.home_dir().join(".aws").join("config");
    let confs: HashMap<_, _> = if aws_config.exists() {
        let config_ini = Ini::load_from_file(&aws_config).context("Can't load aws config")?;
        config_ini
            .iter()
            .map(|(sec, props)| {
                let mut profile = sec.unwrap().to_string();
                if profile.starts_with("profile ") {
                    profile = profile.strip_prefix("profile ").unwrap().to_string();
                }
                let region = props.get("region").unwrap_or_default().to_string();
                let role_arn = props.get("role_arn").unwrap_or_default().to_string();
                let source_profile = props.get("source_profile").unwrap_or_default().to_string();
                (
                    profile.clone(),
                    AwsConfig { profile, region, role_arn, source_profile },
                )
            })
            .collect()
    } else {
        HashMap::new()
    };

    // todo: expand source profiles
    let mut creds: HashMap<_, _> = Ini::load_from_file(&aws_credentials)
        .context("Can't load aws credentials")?
        .iter()
        .filter_map(|(sec, props)| {
            let profile = sec?.to_string();
            let access_key = props.get("aws_access_key_id")?.to_string();
            let secret = props.get("aws_secret_access_key")?.to_string();
            let region = confs.get(&profile).map(|c| c.region.clone())?;
            Some((
                profile.clone(),
                Credential { profile, access_key, secret, region, ..Credential::default() },
            ))
        })
        .collect();

    if let Ok(cache) = File::open(Config::cache_path()) {
        if let Ok(cached_creds) = serde_json::from_reader::<File, Vec<Credential>>(cache) {
            for cc in cached_creds.into_iter().filter(|x| !x.is_expired()) {
                creds.insert(cc.profile.clone(), cc);
            }
        } else {
            p!("Error deserializing cache. Credential cache cleared");
        }
    }

    thread::scope(|scope| {
        let mut threads = Vec::new();
        for (profile, conf) in confs.into_iter() {
            if creds.contains_key(&profile) {
                continue;
            }
            if creds.contains_key(&conf.source_profile) {
                let source_cred = creds[&conf.source_profile].clone();
                threads.push(scope.spawn(move || get_sts_creds(conf, &source_cred)));
            }
        }
        for t in threads {
            match t.join() {
                Ok(Ok(cred)) => _ = creds.insert(cred.profile.clone(), cred),
                Ok(Err(err)) => p!("Error assuming role: {err:#}"),
                Err(_) => p!("A thread panicked"),
            }
        }
    });

    let tosave = creds.values().filter(|x| !x.token.is_empty()).collect_vec();
    match serde_json::to_string(&tosave) {
        Ok(json) => {
            if let Err(err) = std::fs::write(Config::cache_path(), json) {
                p!("Can't write cache: {err:#}");
            }
        }
        Err(err) => p!("Can't serialize cache; {err:#}"),
    }

    Ok(creds.into_values().collect_vec())
}

fn get_credentials() -> Result<Vec<Credential>> {
    if let Ok(cred) = get_env_credentials() {
        return Ok(vec![cred]);
    }
    get_shared_credentials()
}

fn update_from_aws_api(
    keys_path: impl AsRef<Path>,
    cred: &Credential,
    proxy_jump: Option<&str>,
) -> Result<Vec<Instance>> {
    let region = &cred.region;
    let describe_instances = f!("https://ec2.{region}.amazonaws.com/?Action=DescribeInstances&Version=2016-11-15&Filter.1.Name=instance-state-name&Filter.1.Value.1=running");
    let res = aws_send(&describe_instances, "ec2", cred)?;
    if res.status_code == 401 {
        std::fs::remove_file(Config::cache_path())?;
        bail!(
            "Authorization failed for profile {}. Credential cache cleared. Please retry",
            cred.profile
        )
    }
    if res.status_code < 200 || res.status_code > 299 {
        bail!(
            "Error getting instances for {}, {}, {}",
            cred.profile,
            res.status_code,
            res.reason_phrase
        )
    }
    std::fs::write("pippo.xml", res.as_str()?)?;
    let doc = roxmltree::Document::parse(res.as_str()?)?;
    let instances = doc
        .descendants()
        .filter(|n| n.has_tag_name("instancesSet"))
        .filter_map(|i| {
            let instance = i.first_element_child()?;
            let key = instance.find_tag("keyName")?.text()?;
            let key = keys_path.as_ref().join(key).to_str()?.to_string();
            let address_tag = if proxy_jump.is_none_or_empty() { "ipAddress" } else { "privateIpAddress" };
            let address = instance.find_tag(address_tag)?.text()?.to_string();
            let mut tag_set_items = instance.find_tag("tagSet")?.children();
            let tag_name =
                tag_set_items.find_map(|x| (x.find_tag("key")?.text()? == "Name").then_some(x))?;
            let name = tag_name.find_tag("value")?.text()?;
            let name = name.to_string().replace(' ', "-");
            let platform = instance.find_tag("platformDetails")?.text()?.to_string();
            let user = if platform == "Windows" { "administrator" } else { "ubuntu" }.to_string();
            Some(Instance {
                name,
                key,
                address,
                platform,
                user,
                profile: cred.profile.to_owned(),
                proxy_jump: proxy_jump.map(String::from),
            })
        })
        .collect_vec();
    Ok(instances)
}

pub fn update_sshconfig(
    keys_path: impl AsRef<Path>,
    template: impl AsRef<Path>,
    proxy_jump: Option<&str>,
) -> Result<()> {
    let keys_path = keys_path.as_ref();
    let mut srvs: Vec<Instance> = Vec::new();
    let credentials = &get_credentials().context("No credentials found")?;
    ensure!(!credentials.is_empty(), "No credentials found");
    thread::scope(|scope| {
        let threads: Vec<_> = credentials
            .iter()
            .map(|c| {
                scope.spawn(move || {
                    update_from_aws_api(keys_path, c, proxy_jump).context(c.profile.clone())
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
        .context("can't retrieve home directory")?
        .home_dir()
        .join(".ssh")
        .join("config");
    std::fs::write(ssh_config, res)?;

    Ok(())
}
