use clap::Parser;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::str::FromStr;
use std::{fmt, fs};

use crate::cli::common::{self, LockSpec};

#[derive(Debug, Clone)]
pub struct InitError {
    env_path: Option<PathBuf>,
    message: String,
}

impl InitError {
    fn new<T>(p: Option<&PathBuf>, s: T) -> Self
    where
        T: ToString,
    {
        Self {
            env_path: p.map(|res| res.to_path_buf()),
            message: s.to_string(),
        }
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<&str> for InitError {
    fn from(value: &str) -> Self {
        Self::new(None, value)
    }
}

impl From<String> for InitError {
    fn from(value: String) -> Self {
        Self::new(None, value)
    }
}

impl<T> From<(&PathBuf, T)> for InitError
where
    T: ToString,
{
    fn from((env_path, message): (&PathBuf, T)) -> Self {
        Self::new(Some(env_path), message)
    }
}

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
/// If `args.path` contains lockspec files, try to create a new environment. Otherwise,
/// try to use an existing environment.
///
/// * `args`:
pub fn execute(args: Args) {
    println!("initializing env: {:?}", &args.name);
    let cwd = current_dir().unwrap_or_else(|_| {
        eprintln!("Unable to get the current directory.");
        exit(1);
    });
    let path = args
        .path
        .map(|p| {
            PathBuf::from_str(&p).unwrap_or_else(|_| {
                eprintln!("{p} is not a valid path.");
                exit(1);
            })
        })
        .unwrap_or(cwd.clone());

    let result = if LockSpec::from_directory(&path).is_ok() {
        make_new_araki_env(&path, args.name)
    } else {
        use_existing_araki_env(&path, args.name)
    };
    let _ = result.inspect_err(|err| {
        // Delete any araki environment directory if the error specifies such a path to clean up
        eprintln!("{err:?}");
        if let Some(env_path) = &err.env_path {
            let _ = fs::remove_dir_all(env_path).map_err(|fserr| {
                eprintln!("Error cleaning up {env_path:?}.\nReason: {fserr}");
            });
        }
        exit(1);
    });
}

pub fn make_new_araki_env(path: &Path, name: Option<String>) -> Result<(), InitError> {
    let cwd = current_dir().map_err(|_| "Unable to get the current directory.")?;
    let env_name = name
        .or_else(|| cwd.file_name().map(|p| p.to_string_lossy().to_string()))
        .ok_or(
            "No environment name specified, and unable to get the basename of the \
            current directory to infer the new environment name. Aborting.",
        )?;

    if let Ok(_existing_env) = LockSpec::from_env_name(&env_name) {
        return Err(format!(
            "An environment with the name {env_name} already exists. \
                    Please specify a new name."
        )
        .into());
    } else {
        let env_path = common::get_default_araki_envs_dir()
            .map_err(|err| {
                format!("Error getting the araki environment directory.\nReason: {err}")
            })?
            .join(&env_name);
        fs::create_dir(&env_path).map_err(|err| {
            format!("Error creating the araki environment directory.\nReason: {err}")
        })?;

        let path_lockspec = LockSpec::from_directory(path).map_err(|err| (&env_path, err))?;

        println!("Creating new environment {env_name} using existing environment at {path:?}");
        path_lockspec.hardlink_to(&env_path).map_err(|err| {
            (
                &env_path,
                format!(
                    "Hardlinking lockspec files at {path:?} to {env_path:?} failed.\n\
                    Reason: {err}"
                ),
            )
        })?;
        path_lockspec
            .ensure_araki_metadata(&env_name)
            .map_err(|err| (&env_path, err))?;

        let _ = Command::new("pixi")
            .args(["install", "--frozen", "--locked"])
            .current_dir(path)
            .output()
            .map_err(|err| {
                (
                    &env_path,
                    format!("Error running `pixi install --frozen --locked`.\nReason: {err}"),
                )
            })?;
    }
    Ok(())
}

/// Use an existing araki environment in the given path.
///
/// * `path`: Path for which an araki environment is to be used
/// * `name`: Name of the environment to use
pub fn use_existing_araki_env(path: &Path, name: Option<String>) -> Result<(), InitError> {
    let env_name = name.ok_or_else(|| {
        format!(
            "No existing environment name was passed with which to initialize \
            {path:?}. Please specify an existing environment name."
        )
    })?;

    if let Ok(env_lockspec) = LockSpec::from_env_name(&env_name) {
        println!("Using existing environment {env_name} for {path:?}");
        let env_path = common::get_default_araki_envs_dir()
            .map_err(|err| {
                format!("Error getting the araki environment directory.\nReason: {err}")
            })?
            .join(&env_name);

        env_lockspec.hardlink_to(path).map_err(|err| {
            format!(
                "Hardlinking lockspec files at {env_path:?} to {path:?} failed.\n\
                Reason: {err}"
            )
        })?;
        Ok(())
    } else {
        Err(format!("No environment by the name {env_name} exists. Aborting.").into())
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
