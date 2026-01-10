use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

use crate::cli::Cli;

pub fn completion(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    generate(shell, &mut cmd, bin_name, &mut io::stdout());
    Ok(())
}
