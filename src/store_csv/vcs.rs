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
    autocommit: bool,
    autopush: bool,
    #[serde(rename = "type")]
    vcs_type: VcsType,
}

impl Default for VcsSettings {
    fn default() -> Self {
        Self {
            autocommit: true,
            autopush: true,
            vcs_type: VcsType::Git,
        }
    }
}

impl VcsSettings {
    pub(super) fn commit<P: AsRef<Path>>(&self, repo_path: P) -> Result<(), VcsSettingsError> {
        if !self.autocommit {
            return Ok(());
        }

        match self.vcs_type {
            VcsType::Git => {
                debug!("staging all changes in the repo");
                githelper::stage_all(&repo_path)?;

                let message = "Autocommit changes";
                debug!("commiting changes to repo");
                githelper::commit(&repo_path, message)?;

                if self.autopush {
                    debug!("pushing changes to origin");
                    githelper::push_to_origin(&repo_path)?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(super) enum VcsSettingsError {
    GitHelper(githelper::Error),
}

impl fmt::Display for VcsSettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VcsSettingsError::GitHelper(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for VcsSettingsError {
    fn description(&self) -> &str {
        ""
    }
}

impl From<githelper::Error> for VcsSettingsError {
    fn from(err: githelper::Error) -> Self {
        VcsSettingsError::GitHelper(err)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum VcsType {
    Git,
}
