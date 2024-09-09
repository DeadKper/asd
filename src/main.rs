mod cli;
mod config;
mod encryption;

use anyhow::Ok;
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

fn get_cached_password(
    passphrase: &str,
    args: &ConnectionArgs,
    config: &Config,
    dirs: &ConfigDirs,
) -> anyhow::Result<String> {
    if args.ask_pass {
        let pass = scanpw!("Password: ");
        println!();
        return Ok(pass.trim().to_owned());
    }
    let user = args
        .login_name
        .clone()
        .unwrap_or(config.default_login_user.clone());
    let port = args.port.unwrap_or(config.default_login_port);
    let cached_credentials = dirs.state.join(format!("{user}@{}:{port}", args.remote));
    if cached_credentials.exists() {
        return encryption::decrypt(&cached_credentials, Some(passphrase));
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
    let credentials = dirs.data.join("credentials").join(&user);
    if files.is_empty() {
        if credentials.exists() {
            // TODO: password detection
            encryption::decrypt(&credentials, Some(passphrase))
        } else {
            let pass = scanpw!("Password: ");
            println!();
            Ok(pass.trim().to_owned())
        }
    } else {
        let prefix_path = dirs
            .state
            .join(format!("{user}@"))
            .into_os_string()
            .into_string()
            .unwrap();
        let cached: Vec<PathBuf> = files
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
        if !cached.is_empty() {
            encryption::decrypt(&cached[0], Some(passphrase))
        } else {
            encryption::decrypt(&files[0], Some(passphrase))
        }
    }
}

fn ssh(
    passphrase: &str,
    args: &ConnectionArgs,
    config: &Config,
    dirs: &ConfigDirs,
) -> anyhow::Result<()> {
    let user = args
        .login_name
        .clone()
        .unwrap_or(config.default_login_user.clone());
    let port = args.port.unwrap_or(config.default_login_port);
    let cached_credentials = dirs.state.join(format!("{user}@{}:{port}", args.remote));
    let password = get_cached_password(passphrase, args, config, dirs)?;
    encryption::encrypt(passphrase, password.as_bytes(), &cached_credentials)?;
    Ok(())
}
