mod cli;
mod config;
mod encryption;

use anyhow::Ok;
use cli::{CommandEnum, ConfigEnum, Parser};
use config::{Config, ConfigPaths};
use core::panic;
use std::{
    env::args,
    fs,
    io::{self, Write},
    path::Path,
};
use strum::IntoEnumIterator;

#[allow(unused)]
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
    let paths = ConfigPaths::new();
    let passfile = paths.data.join("passphrase.gpg");

    match cli.command {
        CommandEnum::Ssh(args) => {}
        CommandEnum::Sftp(args) => {}
        CommandEnum::Put(args) => {}
        CommandEnum::Get(args) => {}
        CommandEnum::Exec(args) => {}
        CommandEnum::Book(args) => {}
        CommandEnum::Config(command) => match command {
            ConfigEnum::Init => {
                let config_path = paths.config.join("config.toml");
                let mut config = Config::new(&config_path);
                config::create_default_dirs(&paths);
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
                config.default_login_user = register_credentials(&passphrase, user, &paths.data.join("credentials"))?;
                config.save(&config_path)?;
            }
            ConfigEnum::Edit => {
                let path = paths.config.join("config.toml");
                if !path.exists() {
                    Config::reset(&path);
                }
                let config = edit::edit(fs::read_to_string(&path)?)?;
                fs::write(&path, config.as_bytes())?;
            }
            ConfigEnum::Reset => {
                Config::reset(&paths.config.join("config.toml"));
            }
            ConfigEnum::Passphrase => {
                encryption::set_passphrase(&paths.data.join("passphrase.gpg"));
            }
            ConfigEnum::Credentials { user } => {
                register_credentials(
                    &encryption::decrypt(&passfile, None)?,
                    user,
                    &paths.data.join("credentials"),
                );
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
        buffer
    });
    let file = dir.join(&user);
    let buffer = encryption::decrypt(&file, Some(passphrase)).unwrap_or("".to_string());
    let data = edit::edit(buffer)?;
    encryption::encrypt(passphrase, data.as_bytes(), &file)?;
    Ok(user)
}
