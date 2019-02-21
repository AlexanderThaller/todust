use crate::{
    entry::{
        Entries,
        Entry,
    },
    store::Store,
};
use failure::{
    Error,
    ResultExt,
};
use log::debug;
use std::{
    fs,
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

    fn get_entry_filename(&self, entry: &Entry) -> PathBuf {
        let entry_folder = self.get_entry_foldername(&entry);

        let mut entry_file = PathBuf::new();
        entry_file.push(entry_folder);
        entry_file.push(format!("{}.adoc", entry.uuid));

        entry_file
    }
}

impl Store for CsvStore {
    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        let entry_folder = self.get_entry_foldername(&entry);
        fs::create_dir_all(&entry_folder).context("can not create entry folder")?;

        let entry_file = self.get_entry_filename(&entry);

        let mut file = fs::File::create(entry_file).context("can not create entry file")?;
        file.write(entry.text.as_bytes())
            .context("can not write entry text to file")?;

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
