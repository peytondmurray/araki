use clap::Parser;
use std::env::{current_dir, temp_dir};
use std::fs;
use std::path::Path;
use std::process::{Command, exit};
use uuid::Uuid;

use crate::cli::common;

#[derive(Parser, Debug, Default)]
pub struct Args {
    /// Name of the environment. If unspecified, the current directory name is used
    #[arg()]
    name: Option<String>,

    /// Path to the directory to intialize
    #[arg()]
    path: Option<String>,
}

/// Convert the current environment into an araki-managed environment.
///
/// 1. Ensure the user's araki envs dir exists
/// 2.
///
/// * `args`:
pub fn execute(args: Args) {
    println!("initializing env: {:?}", &args.name);

    // Get the araki envs dir
    let araki_envs_dir = common::get_default_araki_envs_dir().unwrap_or_else(|err| {
        eprintln!("Error getting the araki environment directory.\nReason: {err}");
        exit(1);
    });
    let env_name = args.name.unwrap_or_else(|| {
        current_dir()
            .unwrap_or_else(|_| {
                eprintln!("Unable to get the current directory.");
                exit(1);
            })
            .file_name()
            .unwrap_or_else(|| {
                eprintln!("Unable to get the basename of the current directory.");
                exit(1);
            })
            .to_string_lossy()
            .to_string()
    });

    // Check if the project already exists. If it does, exit
    let project_env_dir = araki_envs_dir.join(&env_name);
    if project_env_dir.exists() {
        println!("Using existing environment {env_name}");

        // insert hardlinks here

        eprintln!(
            "Environment {:?} already exists! {project_env_dir:?}",
            &args.name
        );
        exit(1);
    }

    // Since initializing the env repository can fail in a number of different ways,
    // we clone into a temporary directory first. If that's successful, we then move it to the
    // target directory.
    let temp_path = temp_dir().join(Uuid::new_v4().to_string());
    if let Err(err) = fs::create_dir_all(&temp_path) {
        eprintln!("Unable to initialize the repote repository at {temp_path:?}. Reason: {err}",);
        exit(1);
    }
    if let Some(src) = args.repository {
        initialize_remote_git_project(src, &temp_path);
    } else {
        initialize_empty_project(&temp_path);
    }
    if common::copy_directory(&temp_path, &project_env_dir).is_err() {
        eprintln!("Error writing environment to {project_env_dir:?}");
        exit(1);
    }
}

pub fn initialize_remote_git_project(repo: String, project_env_dir: &Path) {
    println!("Pulling from remote repository: {repo:?}");
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

fn hardlink_lockspec(env_dir: &Path, path: &Path) {}
