mod cli;
mod config;

use cli::{CommandEnum, ConfigEnum, Parser};
use config::Config;
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
    let config = Config::new();
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
            ConfigEnum::Init => { }
            ConfigEnum::File => {}
            ConfigEnum::Reset => {
                Config::reset();
            }
            ConfigEnum::Migrate => {}
            ConfigEnum::Passphrase => {}
            ConfigEnum::Credentials { user } => {}
        },
    }
}
