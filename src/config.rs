use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub login_command: HashMap<String, String>,
    pub cached_remote_password_expire_time: String,
}

impl Config {
    pub fn project_dirs() -> ProjectDirs {
        ProjectDirs::from("com.deadkper", "Coppel", "asd")
            .unwrap_or_else(|| panic!("was not able to set project dirs"))
    }

    pub fn config_path() -> PathBuf {
        std::path::Path::new(Self::project_dirs().config_dir()).join("config.toml")
    }

    pub fn new() -> Self {
        let path = Self::config_path();
        if path.exists() {
            Config::load()
        } else {
            Config::reset()
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        toml::from_str::<Config>(&fs::read_to_string(&path).unwrap()).unwrap_or_else(|_| {
            println!("Failed to parse config file, using default configuration");
            Config::default()
        })
    }

    pub fn reset() -> Self {
        let config = Config::default();
        config.save();
        config
    }

    pub fn save(&self) {
        let path = Self::config_path();
        let dir_path = std::path::Path::new(&path).parent().unwrap();
        if !dir_path.exists() {
            match fs::create_dir_all(dir_path) {
                Ok(_) => {}
                Err(error) => {
                    println!("{error}");
                    return;
                }
            };
        }
        match fs::write(path, toml::to_string_pretty(self).unwrap()) {
            Ok(_) => {}
            Err(error) => println!("Error saving config file: {error}"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut login_command: HashMap<String, String> = HashMap::new();
        login_command.insert("root".to_string(), "$SHELL -l".to_string());
        Self {
            cached_remote_password_expire_time: "12h".to_string(),
            login_command
        }
    }
}
