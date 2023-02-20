use clap::Parser;
use clap_complete::Shell;
use directories::UserDirs;
use std::path::PathBuf;

pub const CONFIG_FILE_NAME: &str = "ash.config.json";
pub const DEFAULT_CONFIG: &str = include_str!("../ash.config.json");
pub const COMMON_TSH_ARGS: &[&str] = &["--proxy", "teleport.mago.cloud", "--auth", "github"];
pub const VSDBGSH: &str = include_str!("../res/vsdbg.sh");
pub const VSDBGSH_FILE_NAME: &str = "vsdbg.sh";

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct AshArgs {
    #[arg(name("[profile:]host"), help("Remote Host"))]
    pub host: Option<String>,
    /// Update ssh config
    #[arg(short, long, default_value_t = false)]
    pub update: bool,
    /// Reset to default configuration
    #[arg(long, default_value_t)]
    pub reset: bool,
    /// Clear credentials cache
    #[arg(long, default_value_t)]
    pub clear_cache: bool,
    /// Open config with vscode
    #[arg(long, default_value_t)]
    pub config: bool,
    /// Verbose
    #[arg(long, default_value_t)]
    pub verbose: bool,
    // #[command(subcommand)]
    // pub command: Option<Commands>,
    /// Check for ash update
    #[arg(long, default_value_t = false)]
    pub check_update: bool,

    /// Check for ash update
    #[arg(long, value_enum)]
    pub auto_complete: Option<Shell>,
}

pub struct Config;

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

    pub fn history_path() -> PathBuf {
        Self::config_dir().join("history")
    }

    pub fn code_cmd() -> String {
        if cfg!(windows) { "code.cmd" } else { "code" }.into()
    }
}
