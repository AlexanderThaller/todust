use crate::entry::Metadata;
use log::trace;
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

#[derive(Debug, Clone)]
pub(crate) struct Index {
    folder_path: PathBuf,
    identifier: String,
}

const IDENTIFIER_FILE_EXTENTION: &str = "csv";
const IDENTIFIER_FOLDER_NAME: &str = "identifier";
const INDEX_FILE_NAME: &str = "index.csv";

impl Index {
    /// Create new index from given folder path and use given identifier to
    /// split up the index.
    pub(crate) fn new<P: AsRef<Path>>(folder_path: P, identifier: String) -> Result<Self, Error> {
        fs::create_dir_all(&folder_path)
            .map_err(|err| Error::CreateIndexFolder(folder_path.as_ref().to_path_buf(), err))?;

        Ok(Self {
            folder_path: folder_path.as_ref().to_path_buf(),
            identifier,
        })
    }

    /// Add metadata to index.
    pub(crate) fn metadata_add(&self, metadata: &Metadata) -> Result<(), Error> {
        fs::create_dir_all(self.identifier_folder_path())
            .map_err(|err| Error::CreateIdentifierFolder(self.identifier_folder_path(), err))?;

        let index_path = self.todays_index_path();

        let mut builder = csv::WriterBuilder::new();

        // We only want to write the header if the file does not exist yet so we can
        // just append new entries to the existing file without having multiple
        // headers.
        builder.has_headers(!index_path.exists());

        let index_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&index_path)
            .map_err(|err| Error::OpenIndexFile(index_path.to_path_buf(), err))?;

        let mut writer = builder.from_writer(index_file);

        writer
            .serialize(&metadata)
            .map_err(Error::SerializeMetadata)?;

        Ok(())
    }

    /// Return only most recent metadata. This will be determined based on the
    /// uuid of the entry and the last_change field.
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

    /// Compact files into singular index file and only keep latest state of
    /// entries.
    pub(crate) fn compact(&self) -> Result<(), Error> {
        let metadata = self.metadata_most_recent()?;

        let tmp_dir = tempfile::tempdir().map_err(Error::CompactTempDir)?;
        let tmp_path = tmp_dir.path().join(INDEX_FILE_NAME);

        // In its own scope so the file will be flushed when the scope is closed.
        {
            let tmp_file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&tmp_path)
                .map_err(Error::CompactTempFile)?;

            let builder = csv::WriterBuilder::new();
            let mut writer = builder.from_writer(tmp_file);

            for entry in metadata {
                writer.serialize(&entry).map_err(Error::SerializeMetadata)?;
            }
        }

        let index_file_path = self.folder_path.join(INDEX_FILE_NAME);
        std::fs::copy(tmp_path, index_file_path).map_err(Error::MoveCompactTempFile)?;

        std::fs::remove_dir_all(self.folder_path.join(IDENTIFIER_FOLDER_NAME))
            .map_err(Error::CleanupIdentifierFolder)?;

        Ok(())
    }

    /// Return a list of all projects referenced in the index.
    pub(crate) fn projects(&self) -> Result<Vec<String>, Error> {
        let mut projects = self
            .metadata()?
            .into_iter()
            .map(|metadata| metadata.project)
            .collect::<Vec<_>>();

        projects.sort();
        projects.dedup();

        Ok(projects)
    }

    /// Get all metadata stored in the index.
    /// The index is stored by identifier and current date to make it easier to
    /// sync over git and compact old entries in the future.
    fn metadata(&self) -> Result<BTreeSet<Metadata>, Error> {
        let glob_string = self
            .folder_path
            .join(IDENTIFIER_FOLDER_NAME)
            .join("*")
            .join(format!("*.{}", IDENTIFIER_FILE_EXTENTION));

        let glob = glob::glob(&glob_string.to_string_lossy()).map_err(Error::InvalidGlob)?;

        let mut index_paths = glob
            .collect::<Result<Vec<PathBuf>, glob::GlobError>>()
            .map_err(Error::GlobIteration)?;

        let index_file_path = self.folder_path.join(INDEX_FILE_NAME);
        if index_file_path.exists() {
            index_paths.push(index_file_path);
        }

        trace!("index_paths: {:?}", index_paths);

        let metadata = index_paths
            .into_iter()
            .map(Index::read_metadata_file)
            .collect::<Result<Vec<Vec<_>>, Error>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(metadata)
    }

    /// Deserialize metadata from given path.
    fn read_metadata_file<P: AsRef<Path>>(file_path: P) -> Result<Vec<Metadata>, Error> {
        let file = std::fs::File::open(&file_path)
            .map_err(|err| Error::OpenIndexFile(file_path.as_ref().to_path_buf(), err))?;

        let reader = std::io::BufReader::new(file);

        Index::read_metadata(reader)
            .map_err(|err| Error::ReadIndexFile(file_path.as_ref().to_path_buf(), err))
    }

    /// Deserialize metadata from given reader.
    fn read_metadata<R: std::io::Read>(reader: R) -> Result<Vec<Metadata>, csv::Error> {
        let mut csv_reader = csv::ReaderBuilder::new().from_reader(reader);

        csv_reader
            .deserialize()
            .collect::<Result<Vec<Metadata>, csv::Error>>()
    }

    /// Get todays file to store the index.
    /// Will live under {identifier_file_path}/{Year}-{Month}-{Day}.csv.
    fn todays_index_path(&self) -> PathBuf {
        let mut index_path = self
            .identifier_folder_path()
            .join(chrono::Utc::now().date().to_string());

        index_path.set_extension(IDENTIFIER_FILE_EXTENTION);

        index_path
    }

    /// Get path to identifier folder.
    fn identifier_folder_path(&self) -> PathBuf {
        self.folder_path
            .join(IDENTIFIER_FOLDER_NAME)
            .join(&self.identifier)
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    CleanupIdentifierFolder(std::io::Error),
    CompactTempDir(std::io::Error),
    CompactTempFile(std::io::Error),
    CreateIdentifierFolder(PathBuf, std::io::Error),
    CreateIndexFolder(PathBuf, std::io::Error),
    GlobIteration(glob::GlobError),
    InvalidGlob(glob::PatternError),
    MoveCompactTempFile(std::io::Error),
    OpenIndexFile(PathBuf, std::io::Error),
    ReadIndexFile(PathBuf, csv::Error),
    SerializeMetadata(csv::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::CleanupIdentifierFolder(err) => {
                write!(f, "can not cleanup identifier folder: {}", err)
            }
            Error::CompactTempDir(err) => {
                write!(f, "can not create tmp directory for compaction: {}", err)
            }
            Error::CompactTempFile(err) => {
                write!(f, "can not open tmp file for compaction: {}", err)
            }
            Error::CreateIdentifierFolder(path, err) => write!(
                f,
                "can not create identifier folder at path {:?}: {}",
                path, err
            ),
            Error::CreateIndexFolder(path, err) => write!(
                f,
                "cant not create index folder at path {:?}: {}",
                path, err
            ),
            Error::GlobIteration(err) => write!(f, "can not create glob iterator: {}", err),
            Error::InvalidGlob(err) => write!(f, "got invalid glob iterator: {}", err),
            Error::MoveCompactTempFile(err) => write!(
                f,
                "can not replace index file with compacted tmp file: {}",
                err
            ),
            Error::OpenIndexFile(path, err) => {
                write!(f, "can not open index file at path {:?}: {}", path, err)
            }
            Error::SerializeMetadata(err) => write!(f, "cant not generate metadata: {}", err),
            Error::ReadIndexFile(path, err) => {
                write!(f, "can not read index file from path {:?}: {}", path, err)
            }
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unimplemented!()
    }
}
