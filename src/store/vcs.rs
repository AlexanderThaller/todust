use log::debug;
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    fmt,
    path::Path,
};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct VcsSettings {
    #[serde(rename = "type")]
    vcs_type: VcsType,
}

impl Default for VcsSettings {
    fn default() -> Self {
        Self {
            vcs_type: VcsType::Git,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VcsConfig {
    autocommit: bool,
    autopull: bool,
    autopush: bool,
}

impl Default for VcsConfig {
    fn default() -> Self {
        Self {
            autocommit: true,
            autopull: false,
            autopush: false,
        }
    }
}

impl VcsSettings {
    pub(super) fn commit<P: AsRef<Path>>(
        &self,
        repo_path: P,
        message: &str,
        config: &VcsConfig,
    ) -> Result<(), VcsSettingsError> {
        if !config.autocommit {
            return Ok(());
        }

        match self.vcs_type {
            VcsType::Git => {
                debug!("staging all changes in the repo");
                githelper::add(repo_path.as_ref(), &std::path::PathBuf::from("."))
                    .map_err(VcsSettingsError::Add)?;

                debug!("commiting changes to repo");
                githelper::commit(repo_path.as_ref(), message).map_err(VcsSettingsError::Commit)?;

                if config.autopull {
                    debug!("pulling changes from origin");
                    githelper::pull(repo_path.as_ref()).map_err(VcsSettingsError::Pull)?;
                }

                if config.autopush {
                    debug!("pushing changes to origin");
                    githelper::push(repo_path.as_ref()).map_err(VcsSettingsError::Push)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum VcsSettingsError {
    Add(std::io::Error),
    Commit(std::io::Error),
    Pull(std::io::Error),
    Push(std::io::Error),
}

impl fmt::Display for VcsSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VcsSettingsError::Add(err) => write!(f, "can not add files to git repository: {}", err),

            VcsSettingsError::Commit(err) => {
                write!(f, "can not commit changes to git repository: {}", err)
            }

            VcsSettingsError::Pull(err) => {
                write!(f, "can not pull changes from upstream repository: {}", err)
            }

            VcsSettingsError::Push(err) => {
                write!(f, "can not push changes to upstream repository: {}", err)
            }
        }
    }
}

impl std::error::Error for VcsSettingsError {
    fn description(&self) -> &str {
        ""
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum VcsType {
    Git,
}
