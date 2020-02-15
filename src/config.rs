use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fs,
    io::{
        Read,
        Write,
    },
    path::Path,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub(super) struct Config {
    pub(super) identifier: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            identifier: Uuid::new_v4().to_string(),
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
            let mut file = fs::File::open(file_path).map_err(Error::OpenConfigFile)?;

            let mut data = Vec::new();
            file.read_to_end(&mut data).map_err(Error::ReadConfig)?;
            let configuration = toml::from_slice(&data).map_err(Error::Deserialize)?;

            Ok(configuration)
        }
    }
}

#[derive(Debug)]
pub(super) enum Error {
    CreateConfigFile(std::io::Error),
    Deserialize(toml::de::Error),
    OpenConfigFile(std::io::Error),
    Serialize(toml::ser::Error),
    WriteConfig(std::io::Error),
    ReadConfig(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unimplemented!()
    }
}
