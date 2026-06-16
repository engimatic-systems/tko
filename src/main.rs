// Generated from tko.org. Do not edit by hand.

use std::env;
use std::process::ExitCode;

fn main() -> ExitCode {
    match tko::cli::run_from(env::args_os()) {
        0 => ExitCode::SUCCESS,
        code => ExitCode::from(code as u8),
    }
}
