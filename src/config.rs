use anyhow::Ok;
use directories::{ProjectDirs, UserDirs};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fs, path::PathBuf};

use crate::fatal;

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
            Config::load(path).unwrap_or_default()
        } else {
            Config::reset(path).unwrap_or_default()
        }
    }

    fn load(path: &PathBuf) -> anyhow::Result<Self> {
        Ok(toml::from_str::<Config>(&fs::read_to_string(path)?)?)
    }

    pub fn reset(path: &PathBuf) -> anyhow::Result<Self> {
        let config = Config::default();
        config.save(path)?;
        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let dir_path = std::path::Path::new(path).parent().unwrap();
        if !dir_path.exists() {
            debug!("creating dirs for path: {dir_path:?}");
            fs::create_dir_all(dir_path)?;
        }
        fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut login_command: HashMap<String, String> = HashMap::new();
        login_command.insert("root".to_string(), "$SHELL -l".to_string());
        Self {
            ssh_options: [
                "BatchMode=no",
                "Compression=yes",
                "ConnectionAttempts=1",
                "ConnectTimeout=10",
                "GlobalKnownHostsFile=/dev/null",
                "LogLevel=info",
                "NumberOfPasswordPrompts=1",
                "PasswordAuthentication=yes",
                "PreferredAuthentications=password",
                "StrictHostKeyChecking=no",
                "UserKnownHostsFile=/dev/null",
            ]
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>(),
            default_login_port: 22,
            default_login_user: "root".to_string(),
            cached_remote_password_expire_time: "12h".to_string(),
            login_command,
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct ConfigDirs {
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub document: PathBuf,
    pub download: PathBuf,
}

impl ConfigDirs {
    pub fn new() -> Self {
        let proj_dirs = ProjectDirs::from("com.deadkper", "Coppel", "asd")
            .unwrap_or_else(|| fatal!("Was not able to set project dirs structure"));
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
