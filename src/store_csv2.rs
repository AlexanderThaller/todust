use crate::{
    store::Store,
    todo::{
        Entries,
        Entry,
    },
};
use failure::Error;
use std::path::{
    Path,
    PathBuf,
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
}

impl Store for CsvStore {
    fn add_entry(&self, _entry: Entry) -> Result<(), Error> {
        unimplemented!()
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
