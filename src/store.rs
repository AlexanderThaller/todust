use crate::entry::{
    Entries,
    Entry,
    Metadata,
    ProjectCount,
};
use failure::Error;
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) trait Store {
    fn add_entry(&self, entry: Entry) -> Result<(), Error>;
    fn add_metadata(&self, metadata: Metadata) -> Result<(), Error>;
    fn entry_done(&self, entry_id: usize, project: &str) -> Result<(), Error>;
    fn get_active_entries(&self, project: &str) -> Result<Entries, Error>;
    fn get_done_entries(&self, project: &str) -> Result<Entries, Error>;
    fn get_all_entries(&self) -> Result<Entries, Error>;
    fn get_entries(&self, project: &str) -> Result<Entries, Error>;
    fn get_entry_by_id(&self, entry_id: usize, project: &str) -> Result<Entry, Error>;
    fn get_entry_by_uuid(&self, uuid: &Uuid) -> Result<Entry, Error>;
    fn get_latest_metadata(&self) -> Result<Vec<Metadata>, Error>;
    fn get_metadata(&self) -> Result<BTreeSet<Metadata>, Error>;
    fn get_projects_count(&self) -> Result<Vec<ProjectCount>, Error>;
    fn get_projects(&self) -> Result<Vec<String>, Error>;
    fn run_cleanup(&self) -> Result<(), Error>;
    fn update_entry(&self, entry: Entry) -> Result<(), Error>;
}
