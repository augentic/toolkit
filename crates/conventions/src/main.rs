//! Writes and checks Augentic's shared conventions in a consuming repository.
//!
//! `conventions sync` writes every file the toolkit's `conventions/manifest.toml`
//! manages, from the tree embedded at build time or from a checkout named with
//! `--toolkit`. `conventions check` reports each managed file that differs from
//! what `sync` would write, with a unified diff, and each structural rule the
//! repository breaks, and exits 1 when there is anything to report.

mod consumer;
mod manifest;
mod marker;
mod pin;
mod plan;
mod rules;
mod table;
mod template;
mod toolkit;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};

use crate::consumer::Consumer;
use crate::plan::Plan;
use crate::toolkit::Toolkit;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The repository to operate on; the current directory when omitted.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,

    /// Read `<dir>/conventions` from a toolkit checkout instead of the embedded
    /// tree, with this program's version as the pin.
    #[arg(long, global = true, value_name = "DIR")]
    toolkit: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write every managed file the way the toolkit has it.
    Sync,
    /// Report every managed file that differs and every rule broken; exit 1 if any.
    Check,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool> {
    let toolkit = match &cli.toolkit {
        Some(dir) => Toolkit::checkout(dir)?,
        None => Toolkit::embedded(),
    };
    let consumer = Consumer::open(&cli.root)
        .with_context(|| format!("reading the repository at {}", cli.root.display()))?;
    let plan = Plan::build(&toolkit, &consumer)?;

    match cli.command {
        Command::Sync => plan.sync(&cli.root),
        Command::Check => Ok(plan.check(&toolkit, &consumer)),
    }
}
