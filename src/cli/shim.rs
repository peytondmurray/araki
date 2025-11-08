use clap::Parser;
use std::{env, process::Command};

use crate::cli::common::get_default_araki_bin_dir;

#[derive(Parser, Debug)]
pub struct Args {
    args: Vec<String>
}

fn strip_araki_shim_path(path: String) -> Result<String, String> {
    let araki_bin_dir = get_default_araki_bin_dir()?;
    Ok(
        path
            .split(":")
            .skip_while(|item| **item == araki_bin_dir)
            .collect()
    )
}

pub fn execute(args: Args) {
    let value  = env::var("ARAKI_OVERRIDE_SHIM").unwrap_or("false".to_string());
    if value == "1" || value.to_lowercase().trim() == "true" {
        // Run the requested command using the modified PATH
        let current_path = env::var_os("PATH")
            .and_then(|path| path.into_string().ok());

        let mut command = Command::new("pip");
        if let Some(path) = current_path {
            match strip_araki_shim_path(path) {
                Ok(new_env) => command.env("PATH", new_env),
                Err(err) => {
                    eprintln!("Unable to strip the araki shim path from PATH:\n{err}");
                    return;
                }
            };
        }
        let _ = command
            .spawn()
            .map_err(|err| eprintln!("{err}"));
    } else {
        eprintln!(
            "Unable to run `{args:?}`; use araki for environment management. \
            To use the {args:?} anyway, set ARAKI_OVERRIDE_SHIM=1."
        )
    }
}
