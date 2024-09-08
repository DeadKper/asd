mod cli;
mod config;
mod encryption;

use cli::{CommandEnum, ConfigEnum, Parser};
use config::{Config, ConfigPaths};
use std::{env::args, fs, path::Path};
use strum::IntoEnumIterator;

fn main() {
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
    let config = Config::new(&paths.config.join("config.toml"));
    let passfile = paths.data.join("passphrase.gpg");
    println!("{paths:#?}");
    println!("{config:#?}");
    println!("{cli:#?}");

    #[allow(unused)]
    match cli.command {
        CommandEnum::Ssh(args) => {}
        CommandEnum::Sftp(args) => {}
        CommandEnum::Put(args) => {}
        CommandEnum::Get(args) => {}
        CommandEnum::Exec(args) => {}
        CommandEnum::Book(args) => {}
        CommandEnum::Config(command) => match command {
            ConfigEnum::Init => {
                config::create_default_dirs(&paths);
                let passphrase = if !passfile.exists() {
                    encryption::set_passphrase(&passfile)
                } else {
                    encryption::decrypt(&passfile, None).unwrap()
                };
                register_credentials(&passphrase, None, &paths.data.join("credentials"));
            }
            ConfigEnum::Edit => {
                let path = paths.config.join("config.toml");
                if !path.exists() {
                    Config::reset(&path);
                }
                let config = edit::edit(fs::read_to_string(&path).unwrap())
                    .expect("Cannot read config file");
                fs::write(&path, config.as_bytes()).expect("Unable to write changes");
            }
            ConfigEnum::Reset => {
                Config::reset(&paths.config.join("config.toml"));
            }
            ConfigEnum::Passphrase => {
                encryption::set_passphrase(&paths.data.join("passphrase.gpg"));
            }
            ConfigEnum::Credentials { user } => {
                register_credentials(
                    &encryption::decrypt(&passfile, None).unwrap(),
                    user,
                    &paths.data.join("credentials"),
                );
            }
        },
    }
}

fn register_credentials(passphrase: &str, user: Option<String>, dir: &Path) -> String {
    let user: String = user.unwrap_or_else(|| {
        print!("Enter user to register credentials: ");
        text_io::read!("{}\n")
    });
    let file = dir.join(&user);
    let buffer = if let Some(str) = encryption::decrypt(&file, Some(passphrase)) {
        str
    } else {
        "".to_string()
    };
    let data = edit::edit(buffer).expect("Cannot get data from default editor");
    encryption::encrypt(passphrase, data.as_bytes(), &file);
    user
}
