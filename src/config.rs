use crate::{
    executable::{ScpArgs, TunnelArgs},
    prelude::*,
};
use clap::{Parser, Subcommand, ValueEnum};
use directories::UserDirs;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    fs::File,
    path::PathBuf,
    process::{exit, Command},
};

pub const CONFIG_FILE_NAME: &str = "ash.config.json";
pub const TEMPLATE_FILE_NAME: &str = "template.for.sshconfig.hbs";
pub const DEFAULT_TEMPLATE: &str = include_str!("../res/template.for.sshconfig.hbs");
pub const DEFAULT_CONFIG: &str = include_str!("../ash.config.json");
pub const COMMON_SSH_ARGS: &[&str] = &[
    "-o",
    "StrictHostKeyChecking=no",
    "-o",
    "UserKnownHostsFile=/dev/null",
];

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct AshArgs {
    #[arg(name("[profile:]host"), help("Remote Host"))]
    pub host: Option<String>,
    /// Update ssh config
    #[clap(short, long, default_value_t = false)]
    pub update: bool,
    /// Reset to default configuration
    #[clap(long, default_value_t = false)]
    pub reset: bool,
    /// Clear credentials cache
    #[clap(long, default_value_t = false)]
    pub clear_cache: bool,
    /// Open config with vscode
    #[clap(long, default_value_t = false)]
    pub config: bool,
    /// Verbose
    #[clap(long, default_value_t = false)]
    pub verbose: bool,
    /// Setup ssh config with bastion calculated as <bastion>-<profile>
    #[clap(short, long)]
    pub bastion: Option<String>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Service {
    Rdp,
    Redis,
    Rds,
    RabbitMq,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Copy file/folder from remote
    #[command(arg_required_else_help = true)]
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
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub keys_path: String,
    #[serde(default)]
    pub template_file_path: PathBuf,
    #[serde(default)]
    pub bastion_name: Option<String>,
    #[serde(default)]
    pub update: bool,
    #[serde(default)]
    pub merge_profiles: bool,
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

    pub fn load() -> Result<(Config, AshArgs)> {
        let args = AshArgs::parse();
        let config_path = Self::config_path();
        let template_path = Self::template_path();
        if args.reset {
            if let Err(err) = std::fs::remove_dir_all(Self::config_dir()) {
                p!(
                    "can't remove config folder {:?}: {err:?}",
                    Self::config_path()
                );
            }
            exit(0)
        }
        std::fs::create_dir_all(Self::config_dir())?;
        if !config_path.exists() {
            std::fs::write(&config_path, DEFAULT_CONFIG)?;
        }
        if !template_path.exists() {
            std::fs::write(&template_path, DEFAULT_TEMPLATE)?;
        }
        if args.clear_cache {
            std::fs::remove_file(Self::cache_path())?
        }
        if args.config {
            Command::new("code").arg(Self::config_dir()).status()?;
            exit(0)
        }
        let config = File::open(&config_path).context(f!("can't find config: {config_path:?}"))?;
        let mut config: Config =
            serde_json::from_reader(config).context("Error deserializing config")?;
        config.keys_path =
            config.keys_path.replace('~', Self::home_dir().to_str().expect("can't get home dir"));
        config.template_file_path = template_path;
        args.bastion.is_some().then(|| config.bastion_name = args.bastion.clone());
        config.update = config.update || args.update;
        args.verbose.then(|| p!("{config:?}"));

        Ok((config, args))
    }
}

pub static CFG: Lazy<(Config, AshArgs)> = Lazy::new(|| Config::load().expect("Can't load config"));
