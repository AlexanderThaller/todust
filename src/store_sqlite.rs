use chrono::Utc;
use failure::{
    Error,
    ResultExt,
};
use helper::confirm;
use rusqlite::Connection;
use std::path::PathBuf;
use store::Store;
use todo::{
    Entries,
    Entry,
};
use uuid::Uuid;

#[derive(Default)]
pub struct SqliteStore {
    datafile_path: PathBuf,
}

impl Store for SqliteStore {
    fn with_datafile_path(self, datafile_path: PathBuf) -> Self {
        Self { datafile_path }
    }

    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        let conn = self.connect_database()?;

        conn.execute(
            include_str!("../resources/sqlite/add_entry.sql"),
            &[
                &entry.project_name,
                &entry.started,
                &entry.finished,
                &entry.uuid.to_string(),
                &entry.text,
            ],
        ).context("can not insert entry")?;

        Ok(())
    }

    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error> {
        let conn = self.connect_database()?;

        conn.execute(
            include_str!("../resources/sqlite/update_entry.sql"),
            &[
                &new.project_name,
                &new.started,
                &new.finished,
                &new.text,
                &old.uuid.to_string(),
            ],
        ).context("can not update entry entry")?;

        Ok(())
    }

    fn get_entries(&self) -> Result<Entries, Error> {
        let conn = self.connect_database()?;

        let mut stmt = conn
            .prepare(include_str!("../resources/sqlite/get_entries.sql"))
            .context("can not prepare statement to get entries")?;

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

        Ok(entries)
    }

    fn entry_done(&self, entry_id: usize) -> Result<(), Error> {
        let entries = self.get_entries()?;

        let active_entries: Entries = entries
            .clone()
            .into_iter()
            .filter(|entry| entry.is_active())
            .collect();

        trace!(
            "active_entries: {}, entry_id: {}",
            active_entries.len(),
            entry_id
        );

        if active_entries.len() < entry_id {
            bail!("no active entry found with id {}", entry_id)
        }

        let (_, entry) = active_entries
            .into_iter()
            .enumerate()
            .nth(entry_id - 1)
            .unwrap();

        let message = format!("do you want to finish this entry?:\n{}", entry.to_string());
        if !confirm(&message, false)? {
            bail!("not finishing task then")
        }

        let new = Entry {
            finished: Some(Utc::now()),
            ..entry.clone()
        };

        self.update_entry(&entry, new)?;

        Ok(())
    }
}

impl SqliteStore {
    fn connect_database(&self) -> Result<Connection, Error> {
        let conn =
            Connection::open(&self.datafile_path).context("can not open sqlite database file")?;

        conn.execute(include_str!("../resources/sqlite/schema.sql"), &[])
            .context("can not create entries table")?;

        Ok(conn)
    }
}
