use crate::entry::Metadata;
use failure::{
    Error,
    ResultExt,
};
use fs2::FileExt;
use log::trace;
use std::{
    collections::BTreeSet,
    fs::File,
    path::{
        Path,
        PathBuf,
    },
};

pub(crate) struct CsvIndex {
    index_file_path: PathBuf,
}

impl CsvIndex {
    pub(crate) fn new<P: AsRef<Path>>(index_file_path: P) -> Self {
        Self {
            index_file_path: index_file_path.as_ref().to_path_buf(),
        }
    }

    fn open_index_file(&self) -> Result<File, Error> {
        let file = File::open(&self.index_file_path).context("can not open index file")?;
        file.lock_exclusive().context("can not lock index file")?;

        Ok(file)
    }

    pub(super) fn insert_entry(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = &self.index_file_path;

        let mut builder = csv::WriterBuilder::new();

        // We only want to write the header if the file does not exist yet so we can
        // just append new entries to the existing file without having multiple
        // headers.
        builder.has_headers(!index_path.exists());

        let index_file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&index_path)
            .context("can not open index_file")?;

        index_file.lock_exclusive()?;

        let mut writer = builder.from_writer(index_file);

        writer
            .serialize(&entry)
            .context("can not serialize entry to csv")?;

        Ok(())
    }

    pub(super) fn get_projects(&self) -> Result<Vec<String>, Error> {
        let mut projects = self
            .get_latest_metadata()?
            .into_iter()
            .map(|metadata| metadata.project)
            .collect::<Vec<_>>();

        projects.dedup();

        Ok(projects)
    }

    pub(crate) fn get_metadata(&self) -> Result<BTreeSet<Metadata>, Error> {
        let index_path = &self.index_file_path;

        if !index_path.exists() {
            return Ok(BTreeSet::default());
        }

        let file = self.open_index_file()?;
        let mut csv_reader = csv::ReaderBuilder::new().from_reader(&file);

        let entries = csv_reader
            .deserialize()
            .collect::<Result<BTreeSet<Metadata>, csv::Error>>()
            .context("can not deserialize csv for active entries")?;

        Ok(entries)
    }

    pub(crate) fn get_latest_metadata(&self) -> Result<Vec<Metadata>, Error> {
        let raw_entries = self.get_metadata()?;

        let mut latest_entries = std::collections::BTreeMap::new();
        for metadata in raw_entries {
            latest_entries.insert(metadata.uuid, metadata);
        }

        let metadata_entries = latest_entries
            .into_iter()
            .map(|(_, metadata)| metadata)
            .collect();

        trace!("metadata_entries: {:#?}", metadata_entries);

        Ok(metadata_entries)
    }

    pub(crate) fn add_metadata(&self, metadata: &Metadata) -> Result<(), Error> {
        self.insert_entry(&metadata)
            .context("can not add metadata to index")?;

        Ok(())
    }

    pub(super) fn cleanup_duplicate_uuids(&self) -> Result<(), Error> {
        let mut metadata = self.get_latest_metadata()?;

        metadata.sort();

        let index_path = &self.index_file_path;
        std::fs::remove_file(index_path).context("can not remove old index file")?;

        for entry in metadata {
            self.add_metadata(&entry)?;
        }

        Ok(())
    }
}
