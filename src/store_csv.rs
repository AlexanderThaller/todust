use csv::{
    Error as CsvError,
    ReaderBuilder,
};
use failure::{
    Error,
    ResultExt,
};
use std::path::PathBuf;
use store::Store;
use todo::{
    Entries,
    Entry,
};

pub struct CsvStore {
    datafile_path: PathBuf,
}

impl CsvStore {
    pub fn with_datafile_path(self, datafile_path: PathBuf) -> Self {
        Self { datafile_path }
    }
}

impl Default for CsvStore {
    fn default() -> Self {
        Self {
            datafile_path: PathBuf::from("todust.csv"),
        }
    }
}

impl Store for CsvStore {
    fn add_entry(&self, _entry: Entry) -> Result<(), Error> {
        unimplemented!()
    }

    fn update_entry(&self, _old: &Entry, _new: Entry) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_entries(&self, _project: Option<&str>) -> Result<Entries, Error> {
        let mut rdr = ReaderBuilder::new()
            .from_path(&self.datafile_path)
            .context("can not create entry reader")?;

        let entries = rdr
            .deserialize()
            .collect::<Result<Entries, CsvError>>()
            .context("can not deserialize csv entries")?;

        Ok(entries)
    }

    fn get_active_entries(&self, _project: Option<&str>) -> Result<Entries, Error> {
        unimplemented!()
    }

    fn entry_done(&self, _entry_id: usize, _project: Option<&str>) -> Result<(), Error> {
        unimplemented!()
    }

    fn get_entry_by_id(&self, _entry_id: usize, _project: Option<&str>) -> Result<Entry, Error> {
        unimplemented!()
    }

    fn get_projects(&self) -> Result<Vec<String>, Error> {
        unimplemented!()
    }
}
