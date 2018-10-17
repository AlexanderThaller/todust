use failure::Error;
use todo::{
    Entries,
    Entry,
};

pub trait Store {
    fn add_entry(&self, entry: Entry) -> Result<(), Error>;
    fn entry_done(&self, entry_id: usize, project: Option<&str>) -> Result<(), Error>;
    fn get_active_count(&self, project: Option<&str>) -> Result<usize, Error>;
    fn get_active_entries(&self, project: Option<&str>) -> Result<Entries, Error>;
    fn get_count(&self, project: Option<&str>) -> Result<usize, Error>;
    fn get_done_count(&self, project: Option<&str>) -> Result<usize, Error>;
    fn get_entries(&self, Option<&str>) -> Result<Entries, Error>;
    fn get_entry_by_id(&self, entry_id: usize, project: Option<&str>) -> Result<Entry, Error>;
    fn get_projects(&self) -> Result<Vec<String>, Error>;
    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error>;
}
