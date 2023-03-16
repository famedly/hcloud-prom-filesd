use std::path::PathBuf;

use clap::{Parser, ValueHint};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub(crate) config: Option<PathBuf>,
}
