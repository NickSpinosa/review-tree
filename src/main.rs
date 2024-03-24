// #![allow(unused)]
use clap::Parser;
use config::Args;
use utils::report_results;
use core::find_review_requests;
mod config;
mod core;
mod git;
mod prelude;
mod utils;

use crate::prelude::*;

fn main() -> AnyhowResult<()> {
    let mut args = Args::parse();
    args.load_config_file()?;
    
    let outputs = find_review_requests(args);
    report_results(outputs);

    Ok(())
}
