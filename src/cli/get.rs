use std::{
    env::current_dir,
    fmt::Display,
    fs::{exists, remove_dir_all},
    process,
};

use crate::cli::common::{self, LockSpec};
use clap::Parser;
use regex::Regex;

#[derive(Parser, Debug, Default)]
#[command(arg_required_else_help = true)]
pub struct Args {
    /// URL or <github org>/<repo name> of the environment to grab
    env: String,
}

#[derive(Debug, Default)]
pub struct RemoteRepo {
    org: Option<String>,
    repo: String,
    domain: Option<String>,
    protocol: Option<String>,
}

impl RemoteRepo {
    /// Render the repository as a git url
    fn as_url(&self) -> String {
        format!(
            "{}{}/{}/{}",
            self.get_protocol(),
            self.get_domain(),
            self.get_org(),
            self.get_repo(),
        )
    }

    /// Render the repository as an ssh URL
    fn as_ssh_url(&self) -> String {
        format!(
            "git@{}:{}/{}.git",
            self.get_domain(),
            self.get_org(),
            self.get_repo(),
        )
    }

    fn get_org(&self) -> String {
        self.org.clone().unwrap_or("openteams-ai".into())
    }
    fn get_repo(&self) -> String {
        self.repo.clone()
    }
    fn get_protocol(&self) -> String {
        self.protocol.clone().unwrap_or("https://".into())
    }
    fn get_domain(&self) -> String {
        self.domain.clone().unwrap_or("github.com".into())
    }
}

impl Display for RemoteRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_url())
    }
}

/// Clone the given environment URL
///
/// * `env`: Remote URL for an environment. If only <org>/<repo> is passed, the repository is
///   assumed to live on github.
fn parse_repo_arg(env: &str) -> Result<RemoteRepo, String> {
    let re = Regex::new(
        r"((?<protocol>(git\+)?https?://)?(?<domain>github\.com)/)?((?<org>\w+)/)?(?<repo>\w+)",
    )
    .map_err(|_| "Invalid regex for processing git url.")?;

    let captures = re
        .captures(env)
        .ok_or("Unrecognized format for repo name or URL: {env}.")?;

    Ok(RemoteRepo {
        protocol: captures
            .name("protocol")
            .map(|name| name.as_str().to_string()),
        domain: captures
            .name("domain")
            .map(|name| name.as_str().to_string()),
        org: captures.name("org").map(|name| name.as_str().to_string()),
        repo: captures
            .name("repo")
            .ok_or("No repo name found in {env}")?
            .as_str()
            .to_string(),
    })
}

pub fn execute(args: Args) {
    let cwd = current_dir().unwrap_or_else(|err| {
        eprintln!("Could not get the current directory: {err}");
        process::exit(1);
    });
    let target_toml = cwd.join("pixi.toml");
    let target_lock = cwd.join("pixi.lock");

    // Check that the target directory has no existing lockspec pixi.lock and pixi.toml
    if exists(&target_toml).is_err() {
        eprintln!("{target_toml:?} already exists. Aborting.");
        process::exit(1);
    }
    if exists(&target_lock).is_err() {
        eprintln!("{target_lock:?} already exists. Aborting.");
        process::exit(1);
    }
    let remote = parse_repo_arg(&args.env).unwrap_or_else(|err| {
        eprintln!("Could not fetch a repository from {}: {err}", &args.env);
        process::exit(1);
    });
    let envs_dir = common::get_default_araki_envs_dir().unwrap_or_else(|err| {
        eprintln!("Could not get the default araki environment directory: {err}");
        process::exit(1);
    });

    // Only git clone if the env directory doesn't exist locally.
    // Keep track of whether we cloned or not for cleanup later on in the event of failure
    let mut did_clone = false;
    let local_envs = common::get_local_envs().unwrap_or_else(|err| {
        eprintln!("Could not get the list of existing araki environments: {err}");
        process::exit(1);
    });
    if !local_envs.iter().any(|env| env == &remote.get_repo()) {
        // Clone the repository to the araki environments directory
        println!("Cloning the environment...");
        common::git_clone(remote.as_ssh_url(), &envs_dir.join(remote.get_repo())).unwrap_or_else(
            |err| {
                eprintln!("Unable to clone the environment: {err}");
                process::exit(1);
            },
        );
        did_clone = true;
    }

    let lockspec = LockSpec::from_env_name(&remote.repo).unwrap_or_else(|_| {
        eprintln!(
            "Unable to get the lockspec for {}. Is pixi.toml or pixi.lock missing from {}/{} ?",
            remote.get_repo(),
            remote.get_org(),
            remote.get_repo()
        );

        if did_clone {
            let env_dir = envs_dir.join(&remote.repo);
            match remove_dir_all(&env_dir) {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => {
                    eprintln!("Unable to remove {:?}: {e}", &env_dir);
                }
            }
        }
        process::exit(1);
    });

    // Hardlink the environment repository's lockspec to the target directory.
    // If this fails, remove the environment repository if it was cloned before
    lockspec.hardlink_to(&cwd).unwrap_or_else(|err| {
        eprintln!("Unable to hardlink {lockspec} to {cwd:?}: {err}");
        if did_clone {
            lockspec
                .remove_lockspec_and_parent_dir()
                .unwrap_or_else(|rmerr| {
                    eprintln!(
                        "Unable to remove environment at {:?}: {rmerr}",
                        lockspec.path
                    );
                });
        }
        process::exit(1);
    });

    // Install the pixi project.
    // If this fails, remove the environment repository if it was cloned before,
    // in addition to the hardlinked files.
    let status = process::Command::new("pixi")
        .args(["install", "--frozen", "--locked", "--color", "always"])
        .current_dir(&cwd)
        .status();

    if status.is_err() || status.is_ok_and(|code| !code.success()) {
        eprintln!("Failed to install the environment with pixi.");
        if did_clone {
            lockspec
                .remove_lockspec_and_parent_dir()
                .unwrap_or_else(|rmerr| {
                    eprintln!(
                        "Unable to remove environment at {:?}: {rmerr}",
                        lockspec.path
                    );
                });
        }
        match LockSpec::from_directory(&cwd) {
            Ok(env_lockspec) => {
                env_lockspec.remove_files().unwrap_or_else(|rmerr| {
                    eprintln!("Unable to clean up the lockspec in {cwd:?}: {rmerr}")
                });
            }
            Err(othererr) => {
                eprintln!("Unable to clean up the lockspec in {cwd:?}: {othererr}")
            }
        };
        process::exit(1);
    }
}
