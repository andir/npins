use std::{env, io::stdout};

use clap::CommandFactory;
use clap_complete::{Shell, generate};

fn main() {
    let mut cmd = npins::opts::Opts::command();
    let mut out = stdout().lock();

    let shell = env::args()
        .nth(1)
        .expect("Expected at least one argument for the shell")
        .as_str()
        .parse::<Shell>()
        .expect("Argument was not a valid shell");

    generate(shell, &mut cmd, "npins", &mut out)
}
