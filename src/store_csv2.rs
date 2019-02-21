use crate::{
    entry_v2::{
        Entries,
        Entry,
        Metadata,
    },
    store_v2::Store,
};
use csv::{
    Error as CsvError,
    ReaderBuilder,
    WriterBuilder,
};
use failure::{
    Error,
    ResultExt,
};
use log::{
    debug,
    trace,
};
use std::{
    collections::BTreeSet,
    fs::{
        self,
        OpenOptions,
    },
    io::{
        Read,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
};

pub struct CsvStore {
    datadir: PathBuf,
}

impl CsvStore {
    pub fn open<P: AsRef<Path>>(datadir: P) -> Self {
        Self {
            datadir: datadir.as_ref().to_path_buf(),
        }
    }

    fn get_entry_foldername(&self, entry: &Metadata) -> PathBuf {
        let uuid = entry.uuid.to_string();
        debug!("uuid: {}", uuid);

        // Gets the first two characters of the uuid. This should never fail so the
        // unwrap is safe.
        let uuid_prefix = &uuid[..uuid.char_indices().nth(2).unwrap().0];
        debug!("uuid_prefix: {}", uuid_prefix);

        // {{ datadir }}/entries/{{ uuid_prefix }}
        let mut folder = PathBuf::new();
        folder.push(&self.datadir);
        folder.push("entries");
        folder.push(uuid_prefix);

        debug!("folder: {:?}", folder);

        folder
    }

    fn get_entry_filename(&self, entry: &Metadata) -> PathBuf {
        let entry_folder = self.get_entry_foldername(&entry);

        let mut entry_file = PathBuf::new();
        entry_file.push(entry_folder);
        entry_file.push(format!("{}.adoc", entry.uuid));

        entry_file
    }

    fn get_active_index_filename(&self) -> PathBuf {
        let mut index_file = PathBuf::new();
        index_file.push(&self.datadir);
        index_file.push("active.csv");

        index_file
    }

    fn get_done_index_filename(&self) -> PathBuf {
        let mut index_file = PathBuf::new();
        index_file.push(&self.datadir);
        index_file.push("done.csv");

        index_file
    }

    fn write_entry_text(&self, entry: &Entry) -> Result<(), Error> {
        let entry_folder = self.get_entry_foldername(&entry.metadata);
        fs::create_dir_all(&entry_folder).context("can not create entry folder")?;

        let entry_file = self.get_entry_filename(&entry.metadata);

        let mut file = fs::File::create(entry_file).context("can not create entry file")?;
        file.write(entry.text.as_bytes())
            .context("can not write entry text to file")?;

        Ok(())
    }

    fn add_entry_to_index(&self, index_path: PathBuf, entry: &Metadata) -> Result<(), Error> {
        let mut builder = WriterBuilder::new();
        // We only want to write the header if the file does not exist yet so we can
        // just append new entries to the existing file without having multiple
        // headers.
        builder.has_headers(!index_path.exists());

        let index_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&index_path)
            .context("can not open index_file")?;

        let mut writer = builder.from_writer(index_file);

        writer
            .serialize(&entry)
            .context("can not serialize entry to csv")?;

        Ok(())
    }

    fn add_entry_to_active_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_active_index_filename();
        self.add_entry_to_index(index_path, entry)
    }

    fn add_entry_to_done_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_done_index_filename();
        self.add_entry_to_index(index_path, entry)
    }

    fn remove_entry_from_index(&self, index_path: PathBuf, entry: &Metadata) -> Result<(), Error> {
        let entries = {
            let mut rdr = ReaderBuilder::new()
                .from_path(&index_path)
                .context("can not open index for reading")?;

            rdr.deserialize()
                .collect::<Result<Vec<Metadata>, CsvError>>()
                .context("can not deserialize metadata csv from index")?
                .into_iter()
                .filter(|index_entry| index_entry.uuid != entry.uuid)
                .collect::<Vec<_>>()
        };

        if entries.is_empty() {
            fs::remove_file(&index_path).context("can not remove file")?;
        } else {
            let mut wtr = WriterBuilder::new()
                .from_path(&index_path)
                .context("can not open index for writing")?;

            entries
                .iter()
                .map(|entry| wtr.serialize(entry))
                .collect::<Result<(), CsvError>>()
                .context("can not serialize entries to file")?;
        };

        Ok(())
    }

    fn remove_entry_from_active_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_active_index_filename();
        self.remove_entry_from_index(index_path, entry)
    }

    fn get_entry_for_metadata(&self, metadata: Metadata) -> Result<Entry, Error> {
        let entry_file = self.get_entry_filename(&metadata);

        let mut file = fs::File::open(entry_file).context("can not open entry file")?;

        let mut text = String::new();
        file.read_to_string(&mut text)
            .context("can not read entry file text")?;

        Ok(Entry { metadata, text })
    }
}

impl Store for CsvStore {
    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        self.write_entry_text(&entry)
            .context("can not write entry text to file")?;

        if entry.metadata.finished.is_none() {
            self.add_entry_to_active_index(&entry.metadata)
                .context("can not add entry to active index")?;
        } else {
            self.add_entry_to_done_index(&entry.metadata)
                .context("can not add entry to done index")?;
        }

        Ok(())
    }

    fn entry_done(&self, entry_id: usize, project: &str) -> Result<(), Error> {
        let entry = self
            .get_entry_by_id(entry_id, project)
            .context("can not get entry from id")?;

        self.remove_entry_from_active_index(&entry.metadata)
            .context("can not remove entry from active index")?;
        self.add_entry_to_done_index(&entry.metadata)
            .context("can not add entry to done index")?;

        Ok(())
    }

    fn get_active_count(&self, _project: &str) -> Result<usize, Error> {
        unimplemented!()
    }

    fn get_active_entries(&self, _project: &str) -> Result<Entries, Error> {
        let index_path = self.get_active_index_filename();

        debug!("index_path: {:?}", index_path);

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(index_path)
            .context("can not build csv reader")?;

        let metadata_entries = rdr
            .deserialize()
            .collect::<Result<Vec<Metadata>, CsvError>>()
            .context("can not deserialize csv for active entries")?;

        trace!("metadata_entries: {:#?}", metadata_entries);

        let entries = metadata_entries
            .into_iter()
            .map(|metadata| self.get_entry_for_metadata(metadata))
            .collect::<Result<BTreeSet<Entry>, Error>>()
            .context("can not get entry for metadata")?;

        trace!("entries: {:#?}", entries);

        Ok(Entries { entries })
    }

    fn get_count(&self, _project: &str) -> Result<usize, Error> {
        unimplemented!()
    }

    fn get_done_count(&self, _project: &str) -> Result<usize, Error> {
        unimplemented!()
    }

    fn get_entries(&self, _project: &str) -> Result<Entries, Error> {
        unimplemented!()
    }

    fn get_entry_by_id(&self, entry_id: usize, project: &str) -> Result<Entry, Error> {
        let entry = self
            .get_active_entries(project)
            .context("can not get project entries")?
            .entry_by_id(entry_id)
            .context("can not get entry by id")?;

        Ok(entry)
    }

    fn get_projects(&self) -> Result<Vec<String>, Error> {
        unimplemented!()
    }

    fn update_entry(&self, _old: &Entry, _new: Entry) -> Result<(), Error> {
        unimplemented!()
    }
}
