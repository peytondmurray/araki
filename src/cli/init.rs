use clap::Parser;
use std::env::{current_dir};
use std::fs::hard_link;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::str::FromStr;

use crate::cli::common::{self, LockSpec};

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
/// 1. Ensure the user's araki `envs_dir` exists; create it if needed
/// 2. If the requested environment doesn't exist in the envs_dir, move it there
/// 3. Hardlink pixi.lock and pixi.toml to the target path
///
/// * `args`:
pub fn execute(args: Args) {
    println!("initializing env: {:?}", &args.name);

    // Get the araki `envs_dir`, current working directory, environment name, and target path
    // for the environment
    let araki_envs_dir = common::get_default_araki_envs_dir().unwrap_or_else(|err| {
        eprintln!("Error getting the araki environment directory.\nReason: {err}");
        exit(1);
    });
    let cwd = current_dir()
        .unwrap_or_else(|_| {
            eprintln!("Unable to get the current directory.");
            exit(1);
        });
    let path = args.path
        .map(|p| {
            PathBuf::from_str(&p).unwrap_or_else(|_| {
                eprintln!("{p} is not a valid path.");
                exit(1);
            })
        })
        .unwrap_or(cwd);

    // let env_name = args.name.unwrap_or_else(|| {
    //     cwd
    //         .file_name()
    //         .unwrap_or_else(|| {
    //             eprintln!("Unable to get the basename of the current directory.");
    //             exit(1);
    //         })
    //         .to_string_lossy()
    //         .to_string()
    // });

    if let Ok(path_lockspec) = LockSpec::from_directory(&path) {
        if let Some(env_name) = args.name {
            // Path has lockspec, and env name is specified
            if let Ok(env_lockspec) = LockSpec::from_env_name(&env_name) {
                eprintln!("{path:?} already contains a lockspec. To modify the {env_name} \
                    environment, use `araki push` and `araki pull`");
            } else {
                println!("Creating new environment {env_name} using existing environment \
                    at {path:?}");
                let env_path = araki_envs_dir.join(env_name);
                path_lockspec.hardlink_to(&env_path).map_err(|err| {
                    eprintln!("Hardlinking lockspec files at {path:?} to {env_path:?} failed.\n\
                        Reason: {err}")
                });
                exit(1);
            }
        } else {
            // Path has lockspec, but no env name is given
            let env_name = args.name.unwrap_or_else(|| {
                cwd
                    .file_name()
                    .unwrap_or_else(|| {
                        eprintln!("Unable to get the basename of the current directory");
                        exit(1);
                    })
                    .to_string_lossy()
                    .to_string()
            });
            if let Ok(env_lockspec) = LockSpec::from_env_name(&env_name) {


                // TODO check if env_lockspec is the same as path_lockspec


                eprintln!("Existing lockspec files found at {path:?}. No environment name \
                    specified, and an existing environment named {env_name} already exists. \
                    To modify the {env_name} environment, use `araki push` and `araki pull`.
                    ");
                exit(1);
            } else {
                println!("Creating new environment {env_name} from {path:?}")
            }
        }



        // let env_name = args.name.unwrap_or_else(|| {
        //     cwd
        //         .file_name()
        //         .unwrap_or_else(|| {
        //             eprintln!("Unable to get the basename of the current directory.");
        //             exit(1);
        //         })
        //         .to_string_lossy()
        //         .to_string()
        // });

    } else if let lockspec = LockSpec::from_env_name(&env_name) {

    } else {
        if args.name.is_some() {
            eprintln!("No lockspec exists in {path:?}, and no environment named {env_name} \
                exists. Specify either an existing environment to use, or a path containing a \
                lockspec to create a new environment.");
        } else {
            eprintln!("No lockspec exists in {path:?}, and no environment can be found with the \
                current working directory name ({env_name}). Specify either an existing \
                environment to use, or a path containing a lockspec to create a new environment.");
        }
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
