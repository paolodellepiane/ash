use crate::prelude::*;
use directories::UserDirs;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, fs::File, path::PathBuf};

pub const CONFIG_FILE_NAME: &str = "ash.config.json";
pub const TEMPLATE_FILE_NAME: &str = "template.for.sshconfig.hbs";
pub const DEFAULT_TEMPLATE: &str = include_str!("../res/template.for.sshconfig.hbs");
pub const DEFAULT_CONFIG: &str = include_str!("../ash.config.json");
pub const COMMON_SSH_ARGS: &[&str] = &[
];

// #[derive(Parser)]
// #[clap(author, version, about, long_about = None)]
// pub struct AshArgs {
//     #[arg(name("[profile:]host"), help("Remote Host"))]
//     pub host: Option<String>,
//     /// Update ssh config
//     #[arg(short, long, default_value_t = false)]
//     pub update: bool,
//     /// Reset to default configuration
//     #[arg(long, default_value_t)]
//     pub reset: bool,
//     /// Clear credentials cache
//     #[arg(long, default_value_t)]
//     pub clear_cache: bool,
//     /// Open config with vscode
//     #[arg(long, default_value_t)]
//     pub config: bool,
//     /// Verbose
//     #[arg(long, default_value_t)]
//     pub verbose: bool,
//     #[command(subcommand)]
//     pub command: Option<Commands>,
// }

// #[derive(Subcommand)]
// pub enum Commands {
//     /// Copy file/folder from remote
//     #[command(arg_required_else_help = true)]
//     Cp(ScpArgs),
//     /// Create a tunnel for a specific service
//     #[command(arg_required_else_help = true)]
//     Tunnel {
//         /// Service name (e.g. rdp, redis...)
//         name: String,
//     },
//     /// Execute a command remotely
//     #[command(arg_required_else_help = true)]
//     Exec {
//         /// Command to execute
//         command: String,
//     },
//     /// Connect vscode to remote host
//     #[command()]
//     Code,
// }

#[derive(Deserialize, Debug, Clone)]
pub struct TunnelConfig {
    #[serde(default)]
    pub run: Vec<String>,
    pub local: Option<u16>,
    pub remote: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub keys_path: String,
    #[serde(default)]
    pub bastion_name: String,
    #[serde(default)]
    pub update: bool,
    #[serde(default)]
    pub merge_profiles: bool,
    #[serde(default)]
    pub tunnels: HashMap<String, TunnelConfig>,
}

impl Config {
    pub fn user_dirs() -> UserDirs {
        UserDirs::new().expect("can't get user dirs")
    }

    pub fn home_dir() -> PathBuf {
        Self::user_dirs().home_dir().to_owned()
    }

    pub fn config_dir() -> PathBuf {
        Self::user_dirs().home_dir().join(".config").join("ash")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join(CONFIG_FILE_NAME)
    }

    pub fn template_path() -> PathBuf {
        Self::config_dir().join(TEMPLATE_FILE_NAME)
    }

    pub fn cache_path() -> PathBuf {
        Self::config_dir().join("cache")
    }

    pub fn load() -> Result<Config> {
        let config_path = Self::config_path();
        // let template_path = Self::template_path();
        // if args.reset {
        //     if config_path.exists() {
        //         std::fs::remove_file(template_path).context("can't reset template")?;
        //         std::fs::remove_file(config_path).context("can't reset config")?;
        //         std::fs::remove_file(Self::cache_path()).context("can't reset cache")?;
        //     }
        //     exit(0)
        // }
        std::fs::create_dir_all(Self::config_dir())?;
        if !config_path.exists() {
            std::fs::write(&config_path, DEFAULT_CONFIG)?;
        }
        if !Self::template_path().exists() {
            std::fs::write(&Self::template_path(), DEFAULT_TEMPLATE)?;
        }
        // if args.clear_cache {
        //     std::fs::remove_file(Self::cache_path()).context("can't clear cache")?;
        // }
        // if args.config {
        //     Command::new("code").arg(Self::config_dir()).status()?;
        //     exit(0)
        // }
        let config = File::open(&config_path).context(f!("can't find config: {config_path:?}"))?;
        let mut config: Config =
            serde_json::from_reader(config).context("Error deserializing config")?;
        config.keys_path =
            config.keys_path.replace('~', Self::home_dir().to_str().expect("can't get home dir"));
        Ok(config)
    }
}

pub static CFG: Lazy<Config> = Lazy::new(|| Config::load().expect("Can't load config"));
