use clap::{Parser, Subcommand};
use config::builder::DefaultState;
use config::{Config, Environment};
use config::{ConfigBuilder, ConfigError, FileFormat};
use std::process::exit;

use crate::cli::auth;
use crate::cli::checkout;
use crate::cli::clone;
use crate::cli::init;
use crate::cli::list;
use crate::cli::pull;
use crate::cli::push;
use crate::cli::shell;
use crate::cli::shim;
use crate::cli::tag;

pub mod backends;
pub mod cli;
pub mod common;

/// Manage and share environments
#[derive(Parser, Debug)]
#[command(author, version, about = "Manage and version pixi environments")]
pub struct Cli {
    // Manage environments
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
#[command(arg_required_else_help = true)]
pub enum Command {
    /// Authenticate with the configured backend
    Auth(auth::Args),

    /// Checkout a tag of an environment
    Checkout(checkout::Args),

    /// Clone a lockspec from a remote repository and install it in the current directory
    Clone(clone::Args),

    /// Create a new araki-managed lockspec from an existing lockspec
    Init(init::Args),

    /// List available tags
    List(list::Args),

    /// Pull changes from the remote repo
    Pull(pull::Args),

    /// Push changes to the remote repo
    Push(push::Args),

    /// Write config to the shell
    Shell(shell::Args),

    /// Shim for pip, uv, conda, pixi. Meant to be called from shims only, to signal to araki
    /// that the user is attempting to use an unsupported env management tool
    #[command(hide = true)]
    Shim(shim::Args),

    /// Save the current version of the environment
    Tag(tag::Args),
}

/// Get the default araki settings
fn default_settings() -> Result<ConfigBuilder<DefaultState>, ConfigError> {
    Config::builder().set_default("backend", "github")
}

/// Get the araki configuration settings. In order, this merges
///
/// 1. Default settings
/// 2. User-level araki.toml
/// 3. Local araki.toml
/// 4. Environment variables prefixed with 'ARAKI_'
fn get_settings() -> Result<Config, ConfigError> {
    let user_config = common::get_project_dir()
        .map_err(|err| ConfigError::Message(format!("{err}")))?
        .config_dir()
        .join("araki")
        .join("config");

    default_settings()?
        .add_source(
            config::File::from_str(&user_config.to_string_lossy(), FileFormat::Toml)
                .required(false),
        )
        .add_source(config::File::new("araki", FileFormat::Toml).required(false))
        .add_source(Environment::with_prefix("ARAKI"))
        .build()
}

#[tokio::main]
pub async fn main() {
    let settings = get_settings().unwrap_or_else(|err| {
        eprintln!("Couldn't get the araki settings: {err}");
        exit(1);
    });

    dbg!(&settings);

    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        match cmd {
            Command::Auth(cmd) => auth::execute(cmd, settings).await,
            Command::Checkout(cmd) => checkout::execute(cmd, settings),
            Command::Clone(cmd) => clone::execute(cmd, settings),
            Command::Init(cmd) => init::execute(cmd, settings).await,
            Command::List(cmd) => list::execute(cmd, settings),
            Command::Pull(cmd) => pull::execute(cmd, settings),
            Command::Push(cmd) => push::execute(cmd, settings),
            Command::Shell(cmd) => shell::execute(cmd, settings),
            Command::Shim(cmd) => shim::execute(cmd, settings),
            Command::Tag(cmd) => tag::execute(cmd, settings),
        }
    } else {
        std::process::exit(2);
    }
}
