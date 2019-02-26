use crate::entry_v2::{
    Entries,
    Entry,
    Metadata,
};
use failure::Error;

pub trait Store {
    fn add_entry(&self, entry: Entry) -> Result<(), Error>;
    fn entry_done(&self, entry_id: usize, project: &str) -> Result<(), Error>;
    fn get_active_count(&self, project: &str) -> Result<usize, Error>;
    fn get_active_entries(&self, project: &str) -> Result<Entries, Error>;
    fn get_count(&self, project: &str) -> Result<usize, Error>;
    fn get_done_count(&self, project: &str) -> Result<usize, Error>;
    fn get_entries(&self, _: &str) -> Result<Entries, Error>;
    fn get_entry_by_id(&self, entry_id: usize, project: &str) -> Result<Entry, Error>;
    fn get_projects(&self) -> Result<Vec<String>, Error>;
    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error>;
    fn get_metadata(&self) -> Result<Vec<Metadata>, Error>;
    fn remove_metadata(&self, metadata: &Metadata) -> Result<(), Error>;
    fn add_metadata(&self, metadata: Metadata) -> Result<(), Error>;
    fn run_cleanup(&self) -> Result<(), Error>;
}