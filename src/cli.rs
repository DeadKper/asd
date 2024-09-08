use clap::{ArgGroup, Args, Subcommand};
use std::path::PathBuf;
use strum::{Display, EnumIter};

#[derive(Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub struct Parser {
    #[command(subcommand)]
    pub command: CommandEnum,
}

#[derive(Debug, Subcommand, EnumIter, Display)]
pub enum CommandEnum {
    /// Open SSH connection to given remote [default]
    Ssh(ConnectionArgs),
    /// Open SFTP connection to given remote
    Sftp(ConnectionArgs),
    /// Execute script or commands on listed remotes
    Exec(ExecuteArgs),
    /// Send list of files to the specified remotes
    Put(FileArgs),
    /// Get list of files to the specified remotes
    Get(FileArgs),
    /// Execute ansible playbook using asd password detection
    Book(PlaybookArgs),
    /// Configure application
    #[command(subcommand)]
    Config(ConfigEnum),
}

#[derive(Debug, Args, Default)]
pub struct ConnectionArgs {
    /// Remote to connect to
    pub remote: String,
    /// Ask for connection password
    #[arg(short = 'k', long)]
    pub ask_pass: bool,
    /// Disabled password renewal (and connectivity test in case of --dry-run)
    #[arg(short, long)]
    pub cache: bool,
    /// Force password renewal, invalidating cache
    #[arg(short, long)]
    pub force: bool,
    /// Do not connect to the remote; merely test the connection
    #[arg(short = 'u', long)]
    pub dry_run: bool,
    /// Print password (implies --dry-run)
    #[arg(short, long)]
    pub print: bool,
    /// Quiet mode. Causes most warning and diagnostic messages to be suppressed
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Verbose mode. Causes asd to print debugging messages
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,
}

#[derive(Debug, Args, Default)]
#[clap(group( ArgGroup::new("required") .required(true)))]
#[clap(group( ArgGroup::new("exec") .required(false)))]
pub struct ExecuteArgs {
    /// Specify inventory host path or comma separated host list
    #[arg(short, long)]
    pub inventory: String,
    /// Script path to execute in the remote
    #[arg(
        short = 'x',
        long,
        value_name = "FILE",
        group = "required",
        group = "exec"
    )]
    pub execute: Option<PathBuf>,
    /// Args to be passed to remote script
    #[clap(short, long, allow_hyphen_values = true, requires = "exec")]
    pub args: Option<Vec<String>>,
    /// Commands to send to the remote
    #[arg(short, long, allow_hyphen_values = true, group = "required")]
    pub commands: Option<Vec<String>>,
    /// Quiet mode. Causes most warning and diagnostic messages to be suppressed
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Verbose mode. Causes asd to print debugging messages
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,
}

#[derive(Debug, Args, Default)]
pub struct FileArgs {
    /// Specify inventory host path or comma separated host list
    #[arg(short, long)]
    pub inventory: String,
    /// Files to send to the remotes
    pub files: Vec<PathBuf>,
    /// Quiet mode. Causes most warning and diagnostic messages to be suppressed
    #[arg(short, long, conflicts_with = "verbose")]
    pub quiet: bool,
    /// Verbose mode. Causes asd to print debugging messages
    #[arg(short, long, conflicts_with = "quiet")]
    pub verbose: bool,
}

#[derive(Debug, Args, Default)]
pub struct PlaybookArgs {
    /// Specify inventory host path or comma separated host list
    #[arg(short, long)]
    pub inventory: String,
    /// Args to be passed to ansible, must be specified after --inventory
    #[clap(trailing_var_arg = true, allow_hyphen_values = true)]
    pub ansible_args: Vec<String>,
}

#[derive(Debug, Subcommand, Default)]
pub enum ConfigEnum {
    /// Initialize configuration and create neccesary folders
    #[default]
    Init,
    /// Create/modify specified credentials
    Credentials { user: Option<String> },
    /// Set/change passphrase (use migrate if you didn't migrate manually)
    Passphrase,
    /// Open config file
    Edit,
    /// Reset config file to the base configuration
    Reset,
}
