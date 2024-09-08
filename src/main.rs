mod cli;

use cli::{Parser, Command, Config};
use std::env;
use strum::IntoEnumIterator;

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
    let cli: Parser = clap::Parser::parse_from(args);

    #[allow(unused)]
    match &cli.command {
        Command::Ssh(args) => {}
        Command::Sftp(args) => {}
        Command::Put(args) => {}
        Command::Get(args) => {}
        Command::Exec(args) => {}
        Command::Book(args) => {}
        Command::Config(command) => match command {
            Config::Init => {}
            Config::File => {}
            Config::Reset => {}
            Config::Migrate => {}
            Config::Passphrase => {}
            Config::Credentials { user } => {}
        },
    }

    print!("{cli:#?}");
}
