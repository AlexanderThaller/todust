use crate::{
    entry_v2::{
        Entries,
        Entry,
    },
    store_v2::Store,
};
use failure::{
    Error,
    ResultExt,
};
use log::debug;
use std::{
    fs::{
        self,
        OpenOptions,
    },
    io::Write,
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

    fn get_entry_foldername(&self, entry: &Entry) -> PathBuf {
        let uuid = entry.metadata.uuid.to_string();
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

    fn get_entry_filename(&self, entry: &Entry) -> PathBuf {
        let entry_folder = self.get_entry_foldername(&entry);

        let mut entry_file = PathBuf::new();
        entry_file.push(entry_folder);
        entry_file.push(format!("{}.adoc", entry.metadata.uuid));

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
        let entry_folder = self.get_entry_foldername(&entry);
        fs::create_dir_all(&entry_folder).context("can not create entry folder")?;

        let entry_file = self.get_entry_filename(&entry);

        let mut file = fs::File::create(entry_file).context("can not create entry file")?;
        file.write(entry.text.as_bytes())
            .context("can not write entry text to file")?;

        Ok(())
    }

    fn add_entry_to_index(&self, index_path: PathBuf, entry: &Entry) -> Result<(), Error> {
        let mut builder = csv::WriterBuilder::new();
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
            .serialize(&entry.metadata)
            .context("can not serialize entry to csv")?;

        Ok(())
    }

    fn add_entry_to_active_index(&self, entry: &Entry) -> Result<(), Error> {
        let index_path = self.get_active_index_filename();
        self.add_entry_to_index(index_path, entry)
    }

    fn add_entry_to_done_index(&self, entry: &Entry) -> Result<(), Error> {
        let index_path = self.get_done_index_filename();
        self.add_entry_to_index(index_path, entry)
    }
}

impl Store for CsvStore {
    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        self.write_entry_text(&entry)?;

        if entry.metadata.finished.is_none() {
            self.add_entry_to_active_index(&entry)?;
        } else {
            self.add_entry_to_done_index(&entry)?;
        }

        Ok(())
    }

    fn entry_done(&self, _entry_id: usize, _project: &str) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_active_count(&self, _project: &str) -> Result<usize, Error> {
        unimplemented!()
    }

    fn get_active_entries(&self, _project: &str) -> Result<Entries, Error> {
        unimplemented!()
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

    fn get_entry_by_id(&self, _entry_id: usize, _project: &str) -> Result<Entry, Error> {
        unimplemented!()
    }

    fn get_projects(&self) -> Result<Vec<String>, Error> {
        unimplemented!()
    }

    fn update_entry(&self, _old: &Entry, _new: Entry) -> Result<(), Error> {
        unimplemented!()
    }
}
