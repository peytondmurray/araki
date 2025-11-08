use directories::UserDirs;
use clap::Parser;
use std::{fmt::{self}, fs::{self, File}, path::Path, str::FromStr};
use std::io::{Write};


#[derive(Parser, Debug, Default)]
pub struct Args {
    /// name of the tag
    #[arg()]
    shell: Option<String>,
}

enum Shell {
    Bash,
    Zsh,
    Unknown(String),
}

#[derive(Debug)]
struct ParseShellError;

impl FromStr for Shell {
    type Err = ParseShellError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let result = match s.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            shell => Shell::Unknown(shell.to_string())
        };

        Ok(result)
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let shell_name = match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Unknown(name) => name,
        };
        write!(f, "{}", shell_name)
    }
}

impl Shell {
    fn maybe_write_posix(&self, path: &Path) -> Result<(), String> {
        const ARAKI_POSIX_CONFIG: &str = "\
            # Araki configuration\n\
            eval $(araki shell generate posix)\n\
        ";
        let contents = fs::read_to_string(path)
            .map_err(|_| "Could not open {path} to read existing shell config.")?;

        if contents.contains(ARAKI_POSIX_CONFIG) {
            return Ok(())
        }

        let mut file = File::options()
            .append(true)
            .open(path)
            .map_err(|_| "Could not open {path} to modify shell config.")?;

        write!(&mut file, "{}", ARAKI_POSIX_CONFIG)
            .map_err(|_| "Unable to write araki shell config to {path}")?;
        Ok(())
    }

    fn update_shell_config(&self) -> Result<(), String> {
        let home_dir = UserDirs::new()
            .ok_or("Could not get the home directory for the system.")?
            .home_dir()
            .to_path_buf();

        match self {
            Shell::Bash => self.maybe_write_posix(&home_dir.join(".bashrc")),
            Shell::Zsh => self.maybe_write_posix(&home_dir.join(".zshrc")),
            Shell::Unknown(shell) => Err(
                format!(
                    "{shell} is not one of the supported shells: {}",
                    Shell::supported_shells().join(", ")
                )
            )
        }
    }

    fn supported_shells() -> Vec<&'static str> {
        vec!["bash", "zsh"]
    }

    fn detect() -> Self {
        let system = sysinfo::System::new_with_specifics(
            sysinfo::RefreshKind::default().with_processes(sysinfo::ProcessRefreshKind::default()),
        );
        let my_pid = sysinfo::get_current_pid().expect("unable to get PID of the current process");
        let parent_pid = system
            .process(my_pid)
            .expect("no self process?")
            .parent()
            .expect("unable to get parent process");
        let parent_process = system
            .process(parent_pid)
            .expect("unable to get parent process");
        let parent_name = parent_process.name().to_string_lossy();
        Self::from_str(&parent_name).unwrap_or(Shell::Unknown("".to_string()))
    }
}

pub fn execute(args: Args) {
    Shell::detect();
    let shell: Shell = args
        .shell
        .map(|name| {
            name
                .parse::<Shell>()
                .unwrap_or_else(|_| unreachable!("All string shell names are valid Shell types"))
        })
        .unwrap_or_else(Shell::detect);

    let _ = shell.update_shell_config();

    println!("Shell configuration updated!")
}
