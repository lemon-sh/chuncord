use color_eyre::{eyre::Result, Report};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, ErrorKind, Read, Write},
    path::PathBuf,
};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub webhooks: HashMap<String, String>,
    pub default_webhook: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        match File::open(get_configpath()?) {
            Ok(mut file) => {
                let mut config_str = String::new();
                file.read_to_string(&mut config_str)?;
                Ok(toml::from_str(&config_str)?)
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {
                let config = Config::default();
                config.save()?;
                Ok(config)
            }
            Err(err) => Err(Report::new(err).wrap_err("Failed to open the config file")),
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_str = toml::to_string_pretty(self)?;
        let mut file = File::create(get_configpath()?)?;
        file.write_all(config_str.as_bytes())?;
        Ok(())
    }
}

fn get_configpath() -> io::Result<PathBuf> {
    let configdir = if let Some(dir) = dirs::config_dir() {
        dir
    } else {
        env::current_dir()?
    }
    .join("dcfzf");
    let configpath = configdir.join("config.toml");
    if !configdir.is_dir() {
        fs::create_dir_all(configdir)?;
    }
    Ok(configpath)
}
