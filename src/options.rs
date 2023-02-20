use crate::prelude::*;
use clap::Parser;
use clap_complete::Shell;
use directories::UserDirs;
use std::path::PathBuf;

pub const CONFIG_FILE_NAME: &str = "ash.config.json";
pub const COMMON_TSH_ARGS: &[&str] = &["--proxy", "teleport.mago.cloud", "--auth", "github"];
pub const VSDBGSH: &str = include_str!("../res/vsdbg.sh");
pub const VSDBGSH_FILE_NAME: &str = "vsdbg.sh";

fn check_ash_update() -> Result<()> {
    if !cfg!(windows) {
        bail!("not implemented on this platform");
    }
    std::process::Command::new("scoop.cmd").args(["update", "-k", "ash"]).spawn()?;
    Ok(())
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct AshArgs {
    #[arg(name("[profile:]host"), help("Remote Host"))]
    pub host: Option<String>,
    /// Reset to default configuration
    #[arg(long, default_value_t)]
    pub reset: bool,
    // #[command(subcommand)]
    // pub command: Option<Commands>,
    /// Check for ash update
    #[arg(long, default_value_t = false)]
    pub check_update: bool,

    /// Check for ash update
    #[arg(long, value_enum)]
    pub auto_complete: Option<Shell>,
}

pub struct Options {
    pub user_dirs: UserDirs,
    pub home_dir: PathBuf,
    pub config_dir: PathBuf,
    pub config_path: PathBuf,
    pub history_path: PathBuf,
    pub code_cmd: String,
    pub vsdbgsh_path: PathBuf,
    pub args: AshArgs,
}

impl Options {
    pub fn new() -> Result<Self> {
        let user_dirs = UserDirs::new().expect("can't get user dirs");
        let home_dir = user_dirs.home_dir().to_owned();
        let config_dir = user_dirs.home_dir().join(".config").join("ash");
        let config_path = config_dir.join(CONFIG_FILE_NAME);
        let history_path = config_dir.join("history");
        let code_cmd = if cfg!(windows) { "code.cmd" } else { "code" }.into();
        let vsdbgsh_path = config_dir.join(VSDBGSH_FILE_NAME);
        let args = AshArgs::parse();
        if args.check_update {
            check_ash_update()?;
            std::process::exit(0)
        }
        if args.reset {
            if config_dir.exists() {
                std::fs::remove_dir_all(&config_dir)?;
            }
            std::process::exit(0)
        }
        std::fs::create_dir_all(&config_dir)?;
        if !vsdbgsh_path.exists() {
            std::fs::write(&vsdbgsh_path, VSDBGSH)?;
        }
        Ok(Self { user_dirs, home_dir, config_dir, config_path, history_path, code_cmd, vsdbgsh_path, args })
    }
}
