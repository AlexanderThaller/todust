use crate::entry::Metadata;
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fs,
    path::{
        Path,
        PathBuf,
    },
};

pub(crate) struct Index {
    folder_path: PathBuf,
    identifier_file_path: PathBuf,
}

const IDENTIFIER_FOLDER_NAME: &str = "identifier";
const INDEX_FILE_NAME: &str = "index.csv";
const IDENTIFIER_FILE_EXTENTION: &str = "csv";

impl Index {
    /// Create new index from given folder path and use given identifier to
    /// split up the index
    pub(crate) fn new<P: AsRef<Path>>(folder_path: P, identifier: &str) -> Result<Self, Error> {
        fs::create_dir_all(&folder_path).map_err(Error::CreateIndexFolder)?;

        let identifier_folder_path = folder_path.as_ref().join(IDENTIFIER_FOLDER_NAME);
        fs::create_dir_all(&identifier_folder_path).map_err(Error::CreateIdentifierFolder)?;

        let mut identifier_file_path = identifier_folder_path.join(identifier);
        identifier_file_path.set_extension(IDENTIFIER_FILE_EXTENTION);

        Ok(Self {
            folder_path: folder_path.as_ref().to_path_buf(),
            identifier_file_path,
        })
    }

    /// Add metadata to index
    pub(crate) fn metadata_add(&self, metadata: &Metadata) -> Result<(), Error> {
        let index_path = &self.identifier_file_path;

        let mut builder = csv::WriterBuilder::new();

        // We only want to write the header if the file does not exist yet so we can
        // just append new entries to the existing file without having multiple
        // headers.
        builder.has_headers(!index_path.exists());

        let index_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&index_path)
            .map_err(Error::OpenIndexFile)?;

        let mut writer = builder.from_writer(index_file);

        writer
            .serialize(&metadata)
            .map_err(Error::SerializeMetadata)?;

        Ok(())
    }

    /// Return only most recent metadata. This will be determined based on the
    /// uuid of the entry and the last_change field
    pub(crate) fn metadata_most_recent(&self) -> Result<BTreeSet<Metadata>, Error> {
        let metadata = self
            .metadata()?
            .into_iter()
            .map(|metadata| (metadata.uuid, metadata))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .map(|(_, metadata)| metadata)
            .collect();

        Ok(metadata)
    }

    /// Compact files into singular index file and deduplicate entries
    pub(crate) fn compact(&self) -> Result<(), Error> {
        unimplemented!()
    }

    /// Return a list of all projects referenced in the index
    pub(crate) fn projects(&self) -> Result<Vec<String>, Error> {
        unimplemented!()
    }

    /// Get all metadata stored in the index
    fn metadata(&self) -> Result<BTreeSet<Metadata>, Error> {
        let glob_string = self
            .folder_path
            .join(IDENTIFIER_FOLDER_NAME)
            .join(format!("*.{}", IDENTIFIER_FILE_EXTENTION));

        dbg!(&glob_string);

        let glob = glob::glob(&glob_string.to_string_lossy()).map_err(Error::InvalidGlob)?;

        let mut index_paths = glob
            .collect::<Result<Vec<PathBuf>, glob::GlobError>>()
            .map_err(Error::GlobIteration)?;

        let index_file_path = self.folder_path.join(INDEX_FILE_NAME);
        if index_file_path.exists() {
            index_paths.push(index_file_path);
        }

        dbg!(&index_paths);

        let metadata = index_paths
            .into_iter()
            .map(std::fs::File::open)
            .collect::<Result<Vec<_>, std::io::Error>>()
            .map_err(Error::OpenIndexFile)?
            .into_iter()
            .map(std::io::BufReader::new)
            .map(Index::read_metadata)
            .collect::<Result<Vec<_>, Error>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(metadata)
    }

    /// Deserialize metadata from given reader
    fn read_metadata<R: std::io::Read>(reader: R) -> Result<Vec<Metadata>, Error> {
        let mut csv_reader = csv::ReaderBuilder::new().from_reader(reader);

        let entries = csv_reader
            .deserialize()
            .collect::<Result<Vec<Metadata>, csv::Error>>()
            .map_err(Error::DeserializeMetadata)?;

        Ok(entries)
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    CreateIdentifierFolder(std::io::Error),
    CreateIndexFolder(std::io::Error),
    DeserializeMetadata(csv::Error),
    OpenIndexFile(std::io::Error),
    SerializeMetadata(csv::Error),
    InvalidGlob(glob::PatternError),
    GlobIteration(glob::GlobError),
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
