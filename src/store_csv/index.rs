use crate::entry::Metadata;
use std::path::{
    Path,
    PathBuf,
};

pub(crate) struct CsvIndex {
    folder_path: PathBuf,
    identifier: String,
}

impl CsvIndex {
    /// Create new index from given folder path and use given identifier to
    /// split up the index
    pub(crate) fn new<P: AsRef<Path>>(folder_path: P, identifier: &str) -> Result<Self, Error> {
        unimplemented!()
    }

    /// Add metadata to index
    pub(crate) fn metadata_add(&self, metadata: &Metadata) -> Result<(), Error> {
        unimplemented!()
    }

    /// Return only most recent metadata. This will be determined based on the
    /// uuid of the entry and the last_change field
    pub(crate) fn metadata_most_recent(&self) -> Result<Vec<Metadata>, Error> {
        unimplemented!()
    }

    /// Compact files into singular index file and deduplicate entries
    pub(crate) fn compact(&self) -> Result<(), Error> {
        unimplemented!()
    }

    /// Return a list of all projects referenced in the index
    pub(crate) fn projects(&self) -> Result<Vec<String>, Error> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub(crate) enum Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        unimplemented!()
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unimplemented!()
    }
}
