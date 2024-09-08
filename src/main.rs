mod cli;
mod config;
mod encryption;

use cli::{CommandEnum, ConfigEnum, Parser};
use config::{ Config, ConfigPaths };
use std::env::args;
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
                let passfile = paths.data.join("passphrase.gpg");
                let passphrase = if !passfile.exists() {
                    encryption::set_passphrase(&passfile)
                } else {
                    encryption::decrypt(&passfile)
                };
            }
            ConfigEnum::Edit => {}
            ConfigEnum::Reset => {
                Config::reset(&paths.config.join("config.toml"));
            }
            ConfigEnum::Migrate => {}
            ConfigEnum::Passphrase => {
                encryption::set_passphrase(&paths.data.join("passphrase.gpg"));
            }
            ConfigEnum::Credentials { user } => {}
        },
    }
}
