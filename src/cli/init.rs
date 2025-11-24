use clap::Parser;
use std::env::temp_dir;
use std::fs;
use std::path::Path;
use std::process::{Command, exit};
use uuid::Uuid;

use crate::backends::{Backend, GitHubBackend};
use crate::cli::common;

#[derive(Parser, Debug, Default)]
pub struct Args {
    /// Name of the environment
    #[arg()]
    name: String,

    /// Path to the directory in which the environment should be initialized
    #[arg(long)]
    path: Option<String>,
}

pub async fn execute(args: Args) {
    println!("initializing env: {:?}", &args.name);

    // Check if the lockspec exists locally, exit if it does
    let envs_dir = common::get_default_araki_envs_dir().unwrap_or_else(|err| {
        eprintln!("Could not get the default araki environment directory: {err}");
        exit(1);
    });
    let env_dir = envs_dir.join(&args.name);
    if env_dir.exists() {
        println!(
            "Environment {:?} already exists at {env_dir:?}.",
            &args.name
        );
        return;
    }

    // Check if the lockspec exists remotely, exit if it does
    let org = "openteams-ai";
    let backend = GitHubBackend::new().unwrap_or_else(|err| {
        eprintln!("Unable to query the git server to check for remote lockspecs: {err}");
        exit(1);
    });
    if backend
        .is_existing_lockspec(org.to_string(), args.name.to_string())
        .await
        .unwrap_or_else(|err| {
            eprintln!("Error checking if this lockspec exists on the remote: {err}");
            exit(1);
        })
    {
        eprintln!(
            "Environment {:?} already exists on the remote environment repository. \
                Try cloning it with `araki get {org}/{}`",
            &args.name, &args.name
        );
        exit(1);
    }

    // // We start by checking if the name of the repo is already taken.
    //
    // // Since initializing the env repository can fail in a number of different ways,
    // // we clone into a temporary directory first. If that's successful, we then move it to the
    // // target directory.
    // let temp_path = temp_dir().join(Uuid::new_v4().to_string());
    // if let Err(err) = fs::create_dir_all(&temp_path) {
    //     eprintln!("Unable to initialize the repote repository at {temp_path:?}. Reason: {err}",);
    //     exit(1);
    // }
    // if let Some(src) = args.repository {
    //     initialize_remote_git_project(src, &temp_path);
    // } else {
    //     initialize_empty_project(&temp_path);
    // }
    // if fs::rename(&temp_path, &env_dir).is_err() {
    //     eprintln!("Error writing environment to {env_dir:?}");
    //     exit(1);
    // }
}

pub fn initialize_remote_git_project(repo: String, project_env_dir: &Path) {
    println!("Pulling from remote repository '{}'", repo);
    let _ = common::git_clone(repo, project_env_dir).map_err(|err| {
        eprintln!("{err}");
        exit(1);
    });

    // TODO: validate that the project has a valid project structure.
    // That means it has a
    //  * pixi.toml or pyproject.toml with pixi config
    //  * pixi.lock

    // Install the pixi project
    let _ = Command::new("pixi")
        .arg("install")
        .current_dir(project_env_dir)
        .output()
        .expect("Failed to execute command");
}

pub fn initialize_empty_project(project_env_dir: &Path) {
    // Initialize the pixi project
    let _ = Command::new("pixi")
        .arg("init")
        .current_dir(project_env_dir)
        .status()
        .expect("Failed to execute command");

    // TODO: change this to use git2
    // Initialize the git repo
    let _ = Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .current_dir(project_env_dir)
        .status()
        .expect("Failed to execute command");

    // Install the pixi project
    let _ = Command::new("pixi")
        .arg("install")
        .current_dir(project_env_dir)
        .status()
        .expect("Failed to execute command");

    // Add initial git commit
    let _ = Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(project_env_dir)
        .status()
        .expect("Failed to execute command");
    let _ = Command::new("git")
        .arg("commit")
        .args(["-m", "\"Initial commit\""])
        .current_dir(project_env_dir)
        .status()
        .expect("Failed to execute command");
}
