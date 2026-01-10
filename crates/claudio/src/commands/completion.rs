use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

use crate::cli::Cli;

pub fn completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "claudio", &mut io::stdout());
    Ok(())
}
