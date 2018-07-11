use chrono::Utc;
use failure::{
    Error,
    ResultExt,
};
use helper::confirm;
use measure::Measure;
use rusqlite::Connection;
use std::path::PathBuf;
use store::Store;
use todo::{
    Entries,
    Entry,
};
use uuid::Uuid;

#[derive(Default)]
pub struct SqliteStore {}

impl SqliteStore {
    pub fn connect_database(self, datafile_path: PathBuf) -> Result<ConnectedSqliteStore, Error> {
        debug!("connecting to database");

        let mut measure = Measure::default();

        let db_connection =
            Connection::open(datafile_path).context("can not open sqlite database file")?;

        trace!("connected to database after {}", measure.duration());

        db_connection
            .execute(include_str!("../resources/sqlite/schema.sql"), &[])
            .context("can not create entries table")?;

        trace!("ran schema query after {}", measure.duration());

        debug!("done connecting to database after {}", measure.done());

        Ok(ConnectedSqliteStore { db_connection })
    }
}

pub struct ConnectedSqliteStore {
    db_connection: Connection,
}

impl Store for ConnectedSqliteStore {
    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        debug!("adding entry");

        let mut measure = Measure::default();

        self.db_connection
            .execute(
                include_str!("../resources/sqlite/add_entry.sql"),
                &[
                    &entry.project_name,
                    &entry.started,
                    &entry.finished,
                    &entry.uuid.to_string(),
                    &entry.text,
                ],
            )
            .context("can not insert entry")?;

        trace!("ran add_entry query after {}", measure.duration());

        debug!("done adding entry after {}", measure.done());

        Ok(())
    }

    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error> {
        debug!("updating entry");

        let mut measure = Measure::default();

        self.db_connection
            .execute(
                include_str!("../resources/sqlite/update_entry.sql"),
                &[
                    &new.project_name,
                    &new.started,
                    &new.finished,
                    &new.text,
                    &old.uuid.to_string(),
                ],
            )
            .context("can not update entry entry")?;

        trace!("ran update_entry query after {}", measure.duration());

        debug!("done updating entry after {}", measure.done());

        Ok(())
    }

    fn get_entries(&self) -> Result<Entries, Error> {
        debug!("getting entries");

        let mut measure = Measure::default();

        let mut stmt = self
            .db_connection
            .prepare(include_str!("../resources/sqlite/get_entries.sql"))
            .context("can not prepare statement to get entries")?;

        trace!("preparted sql after {}", measure.duration());

        let entries =
            stmt.query_map(&[], |row| {
                let uuid_raw: String = row.get(3);
                let uuid = match Uuid::parse_str(&uuid_raw).context("can not parse uuid from row") {
                    Ok(uuid) => uuid,
                    Err(err) => {
                        warn!("can not parse uuid: {}", err);
                        return None;
                    }
                };

                Some(Entry {
                    project_name: row.get(0),
                    started: row.get(1),
                    finished: row.get(2),
                    uuid,
                    text: row.get(4),
                })
            }).context("can not convert rows to entries")?
                .filter_map(|entry| match entry {
                    Ok(entry) => Some(entry),
                    Err(err) => {
                        warn!("problem while getting row from sqlite: {}", err);
                        None
                    }
                })
                .filter_map(|entry| entry)
                .collect();

        trace!("collected entries after {}", measure.duration());

        debug!("done getting entries after {}", measure.done());

        Ok(entries)
    }

    fn get_active_entries(&self) -> Result<Entries, Error> {
        // FIXME: Make this a sqlite query

        let mut measure = Measure::default();

        debug!("getting active entries");

        let entries = self.get_entries()?.get_active();

        trace!("collected active entries after {}", measure.duration());

        debug!("done getting active entries after {}", measure.done());

        Ok(entries)
    }

    fn entry_done(&self, entry_id: usize) -> Result<(), Error> {
        debug!("marking entry as done");

        let mut measure = Measure::default();

        let entry = self
            .get_entry_by_id(entry_id)
            .context(format!("can not get entry with id {}", entry_id))?;

        trace!("got entry after {}", measure.duration());

        let message = format!("do you want to finish this entry?:\n{}", entry.to_string());
        if !confirm(&message, false)? {
            bail!("not finishing task then")
        }

        trace!("user confirmed after {}", measure.duration());

        let new = Entry {
            finished: Some(Utc::now()),
            ..entry.clone()
        };

        self.update_entry(&entry, new)?;

        trace!("updated entry after {}", measure.duration());

        debug!("done marking entry as done after {}", measure.done());

        Ok(())
    }

    fn get_entry_by_id(&self, entry_id: usize) -> Result<Entry, Error> {
        // FIXME: Make this a sqlite query
        debug!("getting entry by id");

        let measure = Measure::default();

        let entry = self.get_entries()?.entry_by_id(entry_id)?;

        debug!("done getting entry by id after {}", measure.done());

        Ok(entry)
    }
}
