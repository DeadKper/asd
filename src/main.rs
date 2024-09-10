mod cli;
mod config;
mod encryption;
mod macros;

use anyhow::bail;
use cli::{CommandEnum, ConfigEnum, ConnectionArgs, Parser};
use config::{Config, ConfigDirs};
use glob::glob;
use log::{debug, info, trace, warn};
use scanpw::scanpw;
use std::{
    env::args,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;

trait UnwrapExit<T> {
    fn unwrap_or_exit(self) -> T;
}

impl<T, E> UnwrapExit<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn unwrap_or_exit(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => fatal!("{e}"),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    let mut args = args().collect::<Vec<_>>();
    trace!("original args: {args:?}");
    if args.len() > 1 {
        // check if default subcommand is needed
        let mut valid = ["help", "-h", "--help", "-V", "--version"]
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        valid.extend(CommandEnum::iter().map(|x| x.to_string().to_lowercase()));
        if !valid.contains(&args[1]) {
            info!(
                "no valid subcommand/flag for first argument ({}) using default subcommand",
                args[1]
            );
            args.insert(1, "ssh".to_string());
            trace!("default subcommand args: {args:?}");
        }
    }
    let cli: Parser = clap::Parser::parse_from(args);
    trace!("parsed args: {cli:?}");
    let dirs = ConfigDirs::new();
    trace!("dirs structure: {dirs:?}");
    let passfile = dirs.data.join("passphrase.gpg");
    let config_path = dirs.config.join("config.toml");

    match cli.command {
        CommandEnum::Ssh(args) => {
            ssh(
                &encryption::get_passphrase(&passfile).unwrap_or_exit(),
                &args,
                &Config::new(&config_path),
                &dirs,
            )
            .unwrap_or_exit();
        }
        CommandEnum::Sftp(_args) => {}
        CommandEnum::Put(_args) => {}
        CommandEnum::Get(_args) => {}
        CommandEnum::Exec(_args) => {}
        CommandEnum::Book(_args) => {}
        CommandEnum::Config(command) => match command {
            ConfigEnum::Init => {
                let mut config = Config::new(&config_path);
                trace!("init: {config:?}");
                let passphrase = if !passfile.exists() {
                    debug!("init: no passfile, creating passphrase file");
                    println!("Creating passphrase");
                    encryption::set_passphrase(&passfile).unwrap_or_exit()
                } else {
                    debug!("init: getting passphrase from passfile");
                    encryption::get_passphrase(&passfile).unwrap_or_exit()
                };
                let user = if config.default_login_user == Config::default().default_login_user {
                    debug!("init: creating default user credentials");
                    None
                } else {
                    debug!("init: using configured user");
                    Some(config.default_login_user)
                };
                config.default_login_user =
                    register_credentials(&passphrase, user, &dirs.data.join("credentials"))
                        .unwrap_or_exit();
                debug!("init: setting credentials user as default");
                config.save(&config_path).unwrap_or_exit();
            }
            ConfigEnum::Edit { file } => match file {
                Some(file) => {
                    debug!("edit: editing user given file");
                    encryption::edit(
                        &file,
                        &encryption::get_passphrase(&passfile).unwrap_or_exit(),
                    )
                    .unwrap_or_exit();
                }
                None => {
                    debug!("edit: editing configuration");
                    let path = dirs.config.join("config.toml");
                    if !path.exists() {
                        debug!("edit: creating default configuration");
                        Config::reset(&path).unwrap_or_exit();
                    }
                    let config_str = fs::read_to_string(&path).unwrap_or_exit();
                    let buffer = edit::edit(&config_str).unwrap_or_exit();
                    if config_str == buffer {
                        warn!("{path:#?} unchanged");
                    } else {
                        debug!("writing config contents");
                        fs::write(&path, buffer.as_bytes()).unwrap_or_exit();
                    }
                }
            },
            ConfigEnum::Reset => {
                Config::reset(&dirs.config.join("config.toml")).unwrap_or_exit();
            }
            ConfigEnum::Passphrase => {
                encryption::set_passphrase(&dirs.data.join("passphrase.gpg")).unwrap_or_exit();
            }
            ConfigEnum::Credentials { user } => {
                register_credentials(
                    &encryption::get_passphrase(&passfile).unwrap_or_exit(),
                    user,
                    &dirs.data.join("credentials"),
                )
                .unwrap_or_exit();
            }
        },
    }
    Ok(())
}

fn register_credentials(
    passphrase: &str,
    user: Option<String>,
    dir: &Path,
) -> anyhow::Result<String> {
    let user: String = user.unwrap_or_else(|| {
        print!("Enter user to register credentials: ");
        io::stdout()
            .flush()
            .unwrap_or_else(|error| fatal!("{error}"));
        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .unwrap_or_else(|error| fatal!("{error}"));
        buffer.trim().to_string()
    });
    let file = dir.join(&user);
    encryption::edit(&file, passphrase)?;
    Ok(user)
}

fn get_cached_file(
    args: &ConnectionArgs,
    config: &Config,
    dirs: &ConfigDirs,
) -> anyhow::Result<PathBuf> {
    let user = args
        .login_name
        .clone()
        .unwrap_or(config.default_login_user.clone());
    let port = args.port.unwrap_or(config.default_login_port);
    let credentials = dirs.state.join(format!("{user}@{}:{port}", args.remote));
    trace!("strict cached filed path: {credentials:?}");
    if credentials.exists() {
        debug!("found strict match cache: {credentials:?}");
        return Ok(credentials);
    }
    let user_glob = if args.login_name.is_some() {
        &user
    } else {
        "*"
    };
    let port_glob = if args.port.is_some() {
        &port.to_string()
    } else {
        "*"
    };
    let glob_pattern = dirs
        .state
        .join(format!("{}@{}:{}", &user_glob, args.remote, &port_glob))
        .into_os_string()
        .into_string()
        .unwrap();
    trace!("glob pattern: {glob_pattern}");
    let files: Vec<PathBuf> = glob(&glob_pattern)?
        .map(|x| x.unwrap_or_else(|err| fatal!("{err}")))
        .collect();
    trace!("glob results: {files:?}");
    if files.is_empty() {
        debug!("no cached files with loose globbing");
        bail!(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "credentials cache not found"
        ));
    } else {
        let prefix_path = dirs
            .state
            .join(format!("{user}@"))
            .into_os_string()
            .into_string()
            .unwrap();
        let cache: Vec<PathBuf> = files
            .clone()
            .into_iter()
            .filter(|file| {
                file.file_name()
                    .unwrap()
                    .to_os_string()
                    .into_string()
                    .unwrap()
                    .starts_with(&prefix_path)
            })
            .collect();
        trace!("filter prefix: {prefix_path}");
        if !cache.is_empty() {
            debug!("found default user cache: {:?}", cache[0]);
            Ok(cache[0].clone())
        } else {
            debug!("found loose cache: {:?}", files[0]);
            Ok(files[0].clone())
        }
    }
}

fn get_password(
    passphrase: &str,
    args: &ConnectionArgs,
    config: &Config,
    dirs: &ConfigDirs,
    cache: Option<&PathBuf>,
) -> anyhow::Result<String> {
    if let Some(cache) = cache {
        debug!("decrypting password cache: {cache:?}");
        encryption::decrypt(passphrase, cache)
    } else {
        if args.cache {
            debug!("forced cache usage but cache was not found");
            bail!(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "credentials cache not found"
            ));
        }
        let user = args
            .login_name
            .clone()
            .unwrap_or(config.default_login_user.clone());
        let credentials = dirs.data.join("credentials").join(&user);
        trace!("saved credentials: {credentials:?}");
        if credentials.exists() {
            debug!("credentials found, testing passwords");
            // TODO: password detection
            let password = encryption::decrypt(passphrase, &credentials)?;
            Ok(password)
        } else {
            debug!("no cache or credentials found, asking user for password");
            let password = scanpw!("Password: ");
            println!();
            Ok(password)
        }
    }
}

fn get_connection_data(
    args: &ConnectionArgs,
    config: &Config,
    cache: Option<&PathBuf>,
) -> anyhow::Result<(String, u16)> {
    let user = if let Some(cache) = cache.as_ref() {
        debug!("getting user from cache");
        cache
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap()
            .split_once("@")
            .unwrap()
            .0
            .to_string()
    } else {
        debug!("getting user from args/config");
        args.login_name
            .clone()
            .unwrap_or(config.default_login_user.clone())
    };
    let port = if let Some(cache) = cache.as_ref() {
        debug!("getting port from cache");
        cache
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap()
            .rsplit_once(":")
            .unwrap()
            .1
            .to_string()
            .parse()?
    } else {
        debug!("getting port from args/config");
        args.port.unwrap_or(config.default_login_port)
    };
    Ok((user, port))
}

fn ssh(
    passphrase: &str,
    args: &ConnectionArgs,
    config: &Config,
    dirs: &ConfigDirs,
) -> anyhow::Result<()> {
    let cache = get_cached_file(args, config, dirs).ok();
    let password = get_password(passphrase, args, config, dirs, cache.as_ref())?;
    let (user, port) = get_connection_data(args, config, cache.as_ref())?;
    if cache.is_none() || args.ask_pass {
        debug!(
            "writing password (cached: {}, ask_pass: {}) for {}@{}:{}",
            cache.is_none(),
            args.ask_pass,
            user,
            args.remote,
            port
        );
        encryption::encrypt(
            passphrase,
            password.as_bytes(),
            &dirs.state.join(format!("{user}@{}:{port}", args.remote)),
        )?;
    }
    if cache.is_some() {
        debug!("password was cached, testing connectivity");
        // TODO: test ssh connection
    }
    if args.print {
        println!("{password}");
        return Ok(());
    }
    if args.dry_run {
        return Ok(());
    }
    // TODO: ssh connection
    Ok(())
}
