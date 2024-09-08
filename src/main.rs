use clap::{ArgGroup, Args, Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use strum::{Display, EnumIter, IntoEnumIterator};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,
    /// Quiet mode. Causes most warning and diagnostic messages to be suppressed.
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,
    /// Verbose mode. Causes asd to print debugging messages about its progress.
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    verbose: bool,
}

#[derive(Debug, Subcommand, EnumIter, Display)]
enum Command {
    /// Open SSH connection to given remote [default]
    Ssh(ConnectionArgs),
    /// Open SFTP connection to given remote
    Sftp(ConnectionArgs),
    /// Execute script or commands on listed remotes
    #[clap(group(
        ArgGroup::new("required")
            .required(true)
    ))]
    #[clap(group(
        ArgGroup::new("exec")
            .required(false)
    ))]
    Exec {
        /// Specify inventory host path or comma separated host list
        #[arg(short, long)]
        inventory: String,
        /// Script path to execute in the remote
        #[arg(
            short = 'x',
            long,
            value_name = "FILE",
            group = "required",
            group = "exec"
        )]
        execute: Option<PathBuf>,
        /// Args to be passed to remote script
        #[clap(short, long, allow_hyphen_values = true, requires = "exec")]
        args: Option<Vec<String>>,
        /// Commands to send to the remote
        #[arg(short, long, allow_hyphen_values = true, group = "required")]
        commands: Option<Vec<String>>,
    },
    /// Send list of files to the specified remotes
    Put(FileArgs),
    /// Get list of files to the specified remotes
    Get(FileArgs),
    /// Execute ansible playbook using asd password detection
    Book {
        /// Specify inventory host path or comma separated host list
        #[arg(short, long)]
        inventory: String,
        /// Args to be passed to ansible, must be specified after --inventory
        #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
        ansible_args: Vec<String>,
    },
    /// Configure application
    #[command(subcommand)]
    Config(ConfigEnum),
}

#[derive(Debug, Subcommand, Default)]
enum ConfigEnum {
    /// Initialize configuration and create neccesary folders
    #[default]
    Init,
    /// Create/modify specified credentials
    Credentials {
        user: Option<String>,
    },
    /// Set/change passphrase (use migrate if you didn't migrate manually)
    Passphrase,
    /// Migrate files with previous password to a new password
    Migrate,
    /// Open config file
    File,
    /// Reset config file to the base configuration
    Reset,
}

#[derive(Debug, Args, Default)]
struct ConnectionArgs {
    /// Remote to connect to
    remote: String,
    /// Ask for connection password
    #[arg(short = 'k', long)]
    ask_pass: bool,
    /// Disabled password renewal (and connectivity test in case of --dry-run)
    #[arg(short, long)]
    cache: bool,
    /// Force password renewal, invalidating cache
    #[arg(short, long)]
    force: bool,
    /// Do not connect to the remote; merely test the connection
    #[arg(short = 'u', long)]
    dry_run: bool,
    /// Print password (implies --dry-run)
    #[arg(short, long)]
    print: bool,
}

#[derive(Debug, Args, Default)]
struct FileArgs {
    /// Specify inventory host path or comma separated host list
    #[arg(short, long)]
    inventory: String,
    /// Files to send to the remotes
    files: Vec<PathBuf>,
}

fn main() {
    let mut args = env::args().collect::<Vec<_>>();
    if args.len() > 1 {
        // check if default subcommand is needed
        let mut valid = ["help", "-h", "--help", "-V", "--version"]
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        valid.extend(Command::iter().map(|x| x.to_string().to_lowercase()));
        if !valid.contains(&args[1]) {
            args.insert(1, "ssh".to_string());
        }
    }
    let cli = Cli::parse_from(args);

    print!("{cli:#?}");
}
