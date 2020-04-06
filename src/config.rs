use crate::store::vcs::VcsConfig;
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fs,
    io::Write,
    path::Path,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub(super) struct Config {
    pub(super) identifier: String,
    pub(super) vcs_config: VcsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            identifier: Uuid::new_v4().to_string(),
            vcs_config: VcsConfig::default(),
        }
    }
}

impl Config {
    pub(super) fn read_path<P: AsRef<Path>>(file_path: P) -> Result<Self, Error> {
        if !file_path.as_ref().exists() {
            let configuration = Self::default();

            let data = toml::to_string_pretty(&configuration).map_err(Error::Serialize)?;

            let mut file = fs::File::create(file_path).map_err(Error::CreateConfigFile)?;
            file.write_all(data.as_bytes())
                .map_err(Error::WriteConfig)?;

            Ok(configuration)
        } else {
            let data: Vec<_> = fs::read(file_path).map_err(Error::ReadConfig)?;
            let configuration = toml::from_slice(&data).map_err(Error::Deserialize)?;

            Ok(configuration)
        }
    }
}

#[derive(Debug)]
pub(super) enum Error {
    CreateConfigFile(std::io::Error),
    Deserialize(toml::de::Error),
    ReadConfig(std::io::Error),
    Serialize(toml::ser::Error),
    WriteConfig(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CreateConfigFile(err) => write!(f, "can not create config file: {}", err),
            Error::Deserialize(err) => write!(f, "problem while parsing config file: {}", err),
            Error::ReadConfig(err) => write!(f, "problem while reading config file: {}", err),
            Error::Serialize(err) => write!(f, "problem while generating config file: {}", err),
            Error::WriteConfig(err) => write!(f, "problem while writing config file: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unimplemented!()
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::CreateConfigFile(err) => Some(err),
            Error::Deserialize(err) => Some(err),
            Error::ReadConfig(err) => Some(err),
            Error::Serialize(err) => Some(err),
            Error::WriteConfig(err) => Some(err),
        }
    }
}
