mod cli;
mod config;
mod encryption;

use anyhow::bail;
use cli::{CommandEnum, ConfigEnum, ConnectionArgs, Parser};
use config::{Config, ConfigDirs};
use core::panic;
use glob::glob;
use scanpw::scanpw;
use std::{
    env::args,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = args().collect::<Vec<_>>();
    if args.len() > 1 {
        // check if default subcommand is needed
        let mut valid = ["help", "-h", "--help", "-V", "--version"]
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        valid.extend(CommandEnum::iter().map(|x| x.to_string().to_lowercase()));
        if !valid.contains(&args[1]) {
            args.insert(1, "ssh".to_string());
        }
    }
    let cli: Parser = clap::Parser::parse_from(args);
    let dirs = ConfigDirs::new();
    let passfile = dirs.data.join("passphrase.gpg");
    let config_path = dirs.config.join("config.toml");

    match cli.command {
        CommandEnum::Ssh(args) => {
            ssh(
                &encryption::decrypt(&passfile, None)?,
                &args,
                &Config::new(&config_path),
                &dirs,
            )?;
        }
        CommandEnum::Sftp(_args) => {}
        CommandEnum::Put(_args) => {}
        CommandEnum::Get(_args) => {}
        CommandEnum::Exec(_args) => {}
        CommandEnum::Book(_args) => {}
        CommandEnum::Config(command) => match command {
            ConfigEnum::Init => {
                let mut config = Config::new(&config_path);
                let passphrase = if !passfile.exists() {
                    encryption::set_passphrase(&passfile)?
                } else {
                    encryption::decrypt(&passfile, None)?
                };
                let user = if config.default_login_user == Config::default().default_login_user {
                    None
                } else {
                    Some(config.default_login_user)
                };
                config.default_login_user =
                    register_credentials(&passphrase, user, &dirs.data.join("credentials"))?;
                config.save(&config_path)?;
            }
            ConfigEnum::Edit => {
                let path = dirs.config.join("config.toml");
                if !path.exists() {
                    Config::reset(&path)?;
                }
                let buffer = fs::read_to_string(&path)?;
                let data = edit::edit(&buffer)?;
                if buffer == data {
                    println!("asd: {path:#?} unchanged")
                } else {
                    fs::write(&path, data.as_bytes())?;
                }
            }
            ConfigEnum::Reset => {
                Config::reset(&dirs.config.join("config.toml"))?;
            }
            ConfigEnum::Passphrase => {
                encryption::set_passphrase(&dirs.data.join("passphrase.gpg"))?;
            }
            ConfigEnum::Credentials { user } => {
                register_credentials(
                    &encryption::decrypt(&passfile, None)?,
                    user,
                    &dirs.data.join("credentials"),
                )?;
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
            .unwrap_or_else(|error| panic!("{error}"));
        let mut buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .unwrap_or_else(|error| panic!("{error}"));
        buffer.trim().to_string()
    });
    let file = dir.join(&user);
    let buffer = encryption::decrypt(&file, Some(passphrase)).unwrap_or("".to_string());
    let data = edit::edit(&buffer)?;
    if buffer == data {
        println!("asd: {file:#?} unchanged")
    } else {
        encryption::encrypt(passphrase, data.as_bytes(), &file)?;
    }
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
    if credentials.exists() {
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
    let files: Vec<PathBuf> = glob(
        dirs.state
            .join(format!("{}@{}:{}", user_glob, args.remote, port_glob))
            .into_os_string()
            .to_str()
            .unwrap(),
    )?
    .map(|x| x.unwrap())
    .collect();
    if files.is_empty() {
        bail!(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Credentials cache not found"
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
        if !cache.is_empty() {
            Ok(cache[0].clone())
        } else {
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
        encryption::decrypt(cache, Some(passphrase))
    } else {
        if args.cache {
            bail!(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Credentials cache not found"
            ));
        }
        let user = args
            .login_name
            .clone()
            .unwrap_or(config.default_login_user.clone());
        let credentials = dirs.data.join("credentials").join(&user);
        if credentials.exists() {
            // TODO: password detection
            let password = encryption::decrypt(&credentials, Some(passphrase))?;
            Ok(password)
        } else {
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
        args.login_name
            .clone()
            .unwrap_or(config.default_login_user.clone())
    };
    let port = if let Some(cache) = cache.as_ref() {
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
        encryption::encrypt(
            passphrase,
            password.as_bytes(),
            &dirs.state.join(format!("{user}@{}:{port}", args.remote)),
        )?;
    }
    if cache.is_some() {
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
