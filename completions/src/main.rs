use std::{env, io::stdout};

use clap::CommandFactory;
use clap_complete::{Shell, generate};

// Alternatively you could turn npins into a library and add this that way
// however at least this way we won't confuse any potential users of libnpins
// and also this can be built ealrier without waiting for npins to finish
#[allow(unused)]
mod opts {
    // rust-analyzer does not like this but its fine!
    // https://github.com/rust-lang/rust-analyzer/issues/20129
    include!("../../src/opts.rs");
}

fn main() {
    let mut cmd = crate::opts::Opts::command();
    let mut out = stdout().lock();

    let shell = env::args()
        .nth(1)
        .expect("Expected at least one argument for the shell")
        .as_str()
        .parse::<Shell>()
        .expect("Argument was not a valid shell");

    generate(shell, &mut cmd, "npins", &mut out)
}
