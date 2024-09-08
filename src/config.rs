use core::panic;
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fs, path::PathBuf};
use whoami::Platform;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub login_command: HashMap<String, String>,
    pub default_login_user: String,
    pub default_login_port: u16,
    pub ssh_options: Vec<String>,
    pub cached_remote_password_expire_time: String,
}

impl Config {
    pub fn new(path: &PathBuf) -> Self {
        if path.exists() {
            Config::load(path)
        } else {
            Config::reset(path)
        }
    }

    pub fn load(path: &PathBuf) -> Self {
        toml::from_str::<Config>(&fs::read_to_string(path).unwrap()).unwrap_or_else(|_| {
            println!("Failed to parse config file, using default configuration");
            Config::default()
        })
    }

    pub fn reset(path: &PathBuf) -> Self {
        let config = Config::default();
        config.save(path);
        config
    }

    pub fn save(&self, path: &PathBuf) {
        let dir_path = std::path::Path::new(path).parent().unwrap();
        if !dir_path.exists() {
            println!("Config dir doesn't exists!, use: asd config init");
        } else {
            match fs::write(path, toml::to_string_pretty(self).unwrap()) {
                Ok(_) => {}
                Err(error) => println!("Error saving config file: {error}"),
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut login_command: HashMap<String, String> = HashMap::new();
        login_command.insert("root".to_string(), "$SHELL -l".to_string());
        let null = if whoami::platform() == Platform::Windows {
            "NUL"
        } else {
            "/dev/null"
        }
        .to_string();
        Self {
            ssh_options: [
                "BatchMode=no",
                "Compression=yes",
                "ConnectionAttempts=1",
                "ConnectTimeout=10",
                &format!("GlobalKnownHostsFile={null}"),
                "LogLevel=info",
                "NumberOfPasswordPrompts=1",
                "PasswordAuthentication=yes",
                "PreferredAuthentications=password",
                "StrictHostKeyChecking=no",
                &format!("UserKnownHostsFile={null}"),
            ]
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>(),
            default_login_port: 22,
            default_login_user: whoami::username(),
            cached_remote_password_expire_time: "12h".to_string(),
            login_command,
        }
    }
}

#[derive(Debug)]
pub struct ConfigPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub document: PathBuf,
    pub download: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> Self {
        let proj_dirs = ProjectDirs::from("com.deadkper", "Coppel", "asd")
            .unwrap_or_else(|| panic!("was not able to set project dirs"));
        let user_dirs = UserDirs::new().unwrap();
        Self {
            data: proj_dirs.data_dir().to_owned(),
            config: proj_dirs.config_dir().to_owned(),
            state: proj_dirs
                .state_dir()
                .unwrap_or(&proj_dirs.data_local_dir().join("state"))
                .to_owned(),
            cache: proj_dirs.cache_dir().to_owned(),
            document: user_dirs.document_dir().unwrap().to_owned(),
            download: user_dirs.download_dir().unwrap().to_owned(),
        }
    }
}

fn create_dir(path: &PathBuf) {
    if !path.exists() {
        match fs::create_dir_all(path) {
            Ok(_) => {}
            Err(error) => {
                println!("{error}");
            }
        };
    }
}

pub fn create_default_dirs(paths: &ConfigPaths) {
    create_dir(&paths.data);
    create_dir(&paths.config);
    create_dir(&paths.state);
    create_dir(&paths.cache);
    create_dir(&paths.document);
    create_dir(&paths.download);
}
