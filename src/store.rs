use failure::Error;
use std::path::PathBuf;
use todo::{
    Entries,
    Entry,
};

pub trait Store {
    fn with_datafile_path(self, datafile_path: PathBuf) -> Self;
    fn add_entry(&self, entry: Entry) -> Result<(), Error>;
    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error>;
    fn get_entries(&self) -> Result<Entries, Error>;
    fn entry_done(&self, entry_id: usize) -> Result<(), Error>;
}
