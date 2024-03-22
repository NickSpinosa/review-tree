#![allow(unused)]
use config::Config;
use dirs::home_dir;
use ignore::WalkBuilder;
use serde::Deserialize;
use core::find_repos;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    str::from_utf8,
};
use thiserror::Error;
mod git;
mod prelude;
mod core;
mod config;
mod utils;

use crate::prelude::*;

fn main() -> AnyhowResult<()> {
    let cfg = Config {
        num_threads: 16,
        root_dir: "/home/nick/code".into(),
        ..Config::default()
    };

    find_repos(cfg);

    Ok(())
}
