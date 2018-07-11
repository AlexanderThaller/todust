use failure::Error;
use todo::{
    Entries,
    Entry,
};

pub trait Store {
    fn add_entry(&self, entry: Entry) -> Result<(), Error>;
    fn entry_done(&self, entry_id: usize) -> Result<(), Error>;
    fn get_active_entries(&self) -> Result<Entries, Error>;
    fn get_entries(&self) -> Result<Entries, Error>;
    fn get_entry_by_id(&self, entry_id: usize) -> Result<Entry, Error>;
    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error>;
}
