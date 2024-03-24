use std::path::PathBuf;

use clap::Parser;
use confy::{get_configuration_file_path, load, store};
use dirs::home_dir;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::AnyhowResult;

const APP_NAME: &str = "gh_review_request";
const CONFIG_NAME: &str = "config";
static DEFAULT_ARGS: Lazy<Args> = Lazy::new(|| Args::default());

#[derive(Parser, Debug, Deserialize, Serialize)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(short = 'd', long, default_value_t = get_home_dir())]
    pub root_dir: String,
    #[arg(short = 't', long, default_value_t = 6)]
    pub num_threads: usize,
    #[arg(short = 's', long, default_value_t = false)]
    pub create_tmux_session: bool,
    #[arg(short = 'w', long, default_value_t = true)]
    pub create_worktree: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            root_dir: get_home_dir(),
            num_threads: 6,
            create_tmux_session: false,
            create_worktree: true
        }
    }
}

impl Args {
    /// Get configuration file.
    /// A new configuration file is created with default values if none exists.
    pub fn load_config_file(&mut self) -> AnyhowResult<&mut Self> {
        let config_file: Self = load(APP_NAME, CONFIG_NAME)?;

        if self.root_dir == DEFAULT_ARGS.root_dir {
            self.root_dir = config_file.root_dir;
        }
        if self.num_threads == DEFAULT_ARGS.num_threads {
            self.num_threads = config_file.num_threads;
        }
        if self.create_tmux_session == DEFAULT_ARGS.create_tmux_session {
            self.create_tmux_session = config_file.create_tmux_session;
        }
        if self.create_worktree == DEFAULT_ARGS.create_worktree {
            self.create_worktree = config_file.create_worktree;
        }

        Ok(self)
    }

    /// Write a configuration file
    /// Updates existing file if one already exists
    #[allow(unused)]
    pub fn write_config_file(&self) -> AnyhowResult<&Self> {
        store(APP_NAME, CONFIG_NAME, self)?;
        Ok(self)
    }

    /// Print configuration file path and its contents
    #[allow(unused)]
    pub fn print_config_file(self) -> AnyhowResult<Self> {
        let file_path: PathBuf = get_configuration_file_path(APP_NAME, None)?;
        println!("Configuration file: '{}'", file_path.display());

        let toml: String = toml::to_string_pretty(&self)?;
        println!("\t{}", toml.replace('\n', "\n\t"));

        Ok(self)
    }
}

fn get_home_dir() -> String {
    if let Some(path) = home_dir() {
        path.to_str().unwrap_or("/").to_owned()
    } else {
        "/".into()
    }
}
