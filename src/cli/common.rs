use directories::UserDirs;
use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, RemoteCallbacks};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub const ARAKI_ENVS_DIR: &str = ".araki/envs";
pub const ARAKI_BIN_DIR: &str = ".araki/bin";

/// Get the user's araki envs directory, which by default
/// is placed in their home directory
pub fn get_default_araki_envs_dir() -> Result<PathBuf, String> {
    let envs_dir = UserDirs::new()
        .ok_or_else(|| "Could not get user directory to store araki environments.")?
        .home_dir()
        .join(ARAKI_ENVS_DIR);

    if !envs_dir.exists() {
        println!("araki envs dir does not exist. Creating it at {envs_dir:?}");
        let _ = fs::create_dir_all(&envs_dir).map_err(|err| {
            format!("Could not create an environment directory at {envs_dir:?}.\nReason: {err}")
        })?;
    }
    Ok(envs_dir)
}

pub fn get_default_araki_bin_dir() -> Result<PathBuf, String> {
    let dir = UserDirs::new()
        .map(|path| path.home_dir().to_path_buf().join(ARAKI_BIN_DIR))
        .ok_or("Could not determine the user home directory.")?;

    if !dir.exists() {
        println!("araki bin dir does not exist. Creating it at {dir:?}");
        fs::create_dir_all(&dir).map_err(|err| {
            eprintln!("Could not create araki bin directory at {dir:?}. Error:\n{err}");
            format!("{err}")
        })?;
    }
    Ok(dir)
}

/// Clone a git repo to a path.
///
/// * `repo`: URL of a git repo to clone
/// * `path`: Path where the repo should be cloned
pub fn git_clone(repo: String, path: &Path) -> Result<(), String> {
    let mut callbacks = RemoteCallbacks::new();

    // Keep track of whether we've tried to get credentials from ssh-agent.
    // See https://github.com/nodegit/nodegit/issues/1133 for an example of this, but it affects
    // git2-rs as well; see https://github.com/rust-lang/git2-rs/issues/1140 and
    // https://github.com/rust-lang/git2-rs/issues/347 for more context.
    let mut tried_agent = false;

    callbacks.credentials(|_url, username_from_url, allowed_types| {
        let username = username_from_url.ok_or(git2::Error::from_str(
            "Unable to get the ssh username from the URL.",
        ))?;
        if tried_agent {
            return Err(git2::Error::from_str(
                "Unable to authenticate via ssh. Is ssh-agent running, and have you \
                    added the ssh key you use for git?",
            ));
        }

        if allowed_types.is_ssh_key() {
            tried_agent = true;
            return Cred::ssh_key_from_agent(username);
        }

        Err(git2::Error::from_str(
            "araki only supports ssh for git interactions. Please configure ssh-agent.",
        ))
    });

    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_opts);

    let _ = builder
        .clone(&repo, path)
        .map_err(|err| format!("Failed to clone {repo}. Reason: {err}"))?;
    Ok(())
}

/// Copy a directory recursively.
///
/// If a problem is encountered during copying, the partially-copied directory
/// will be removed.
///
/// * `from`: Path to be copied
/// * `to`: Destination of the copied directory
pub fn copy_directory(from: &PathBuf, to: &PathBuf) -> std::io::Result<()> {
    if !from.is_dir() {
        return Err(Error::new(
            ErrorKind::NotADirectory,
            format!("{} is not a directory", from.to_string_lossy()),
        ));
    }

    if to.exists() {
        return Err(Error::new(
            ErrorKind::AlreadyExists,
            format!("{} already exists", to.to_string_lossy()),
        ));
    }

    fs::create_dir_all(to)?;
    for item in fs::read_dir(from)? {
        let entry = item?;
        if copy_fs_obj(&from, &to.join(&entry.file_name())).is_err() {
            // Clean up the new directory
            if to.is_dir() {
                fs::remove_dir_all(to);
            }
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unknown issue copying {from:?} to {to:?}."),
            ));
        }
    }
    Ok(())
}

/// Copy a filesystem object from one place to another.
///
/// Directories are copied recursively.
///
/// * `from`: Path to be copied
/// * `to`: Destination of the copied object
fn copy_fs_obj(from: &PathBuf, to: &PathBuf) -> std::io::Result<()> {
    let Some(name) = from.file_name() else {
        return Err(Error::new(
            ErrorKind::Other,
            format!("Can't get filename of {from:?}"),
        ));
    };

    if from.is_dir() {
        copy_directory(&from, &to.join(&name))
    } else {
        let _ = fs::copy(&from, &to.join(&name));
        Ok(())
    }
}
