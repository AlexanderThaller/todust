use crate::{
    entry::{
        Entries,
        Entry,
    },
    measure::Measure,
    store::Store,
};
use failure::{
    Error,
    ResultExt,
};
use log::{
    debug,
    trace,
    warn,
};
use rusqlite::{
    Connection,
    Statement,
    NO_PARAMS,
};
use std::{
    fs,
    path::PathBuf,
};
use uuid::Uuid;

pub struct SqliteStore {
    datafile_path: PathBuf,
}

impl Default for SqliteStore {
    fn default() -> Self {
        Self {
            datafile_path: PathBuf::from("todust.sqlite"),
        }
    }
}

impl SqliteStore {
    pub fn with_datafile_path(self, datafile_path: PathBuf) -> Self {
        Self { datafile_path }
    }

    pub fn open(self) -> Result<OpenSqliteStore, Error> {
        debug!("connecting to database");

        let mut measure = Measure::default();

        fs::create_dir_all(
            &self
                .datafile_path
                .parent()
                .expect("can not get parent folder of sqlite database"),
        )
        .context("can not create folder for sqlite database file")?;

        let db_connection =
            Connection::open(&self.datafile_path).context("can not open sqlite database file")?;

        trace!("connected to database after {}", measure.duration());

        db_connection
            .execute(include_str!("../resources/sqlite/schema.sql"), NO_PARAMS)
            .context("can not create entries table")?;

        trace!("ran schema query after {}", measure.duration());

        self.migration(&db_connection)
            .context("can not run migration")?;

        trace!("ran migration after {}", measure.duration());

        debug!("done connecting to database after {}", measure.done());

        Ok(OpenSqliteStore { db_connection })
    }

    fn migration(&self, db_connection: &Connection) -> Result<(), Error> {
        self.migration_v1_null_project_to_default_project(db_connection)?;

        Ok(())
    }

    fn migration_v1_null_project_to_default_project(
        &self,
        db_connection: &Connection,
    ) -> Result<(), Error> {
        debug!("running migration v1 null_project_to_default_project");

        let measure = Measure::default();

        db_connection
            .execute(
                include_str!(
                    "../resources/sqlite/migration/v1/null_project_to_default_project.sql"
                ),
                NO_PARAMS,
            )
            .context("can not run v1 migration null_project_to_default_project")?;

        debug!(
            "done running migration v1 null_project_to_default_project after {}",
            measure.done()
        );

        Ok(())
    }
}

pub struct OpenSqliteStore {
    db_connection: Connection,
}

impl Store for OpenSqliteStore {
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

    fn get_entries(&self, project: &str) -> Result<Entries, Error> {
        debug!("getting entries");

        let mut measure = Measure::default();

        let stmt = self
            .db_connection
            .prepare(include_str!("../resources/sqlite/get_entries.sql"))
            .context("can not prepare statement to get entries")?;

        trace!("preparted sql after {}", measure.duration());

        let entries = sqlite_statement_to_entries(stmt, project)
            .context("can not convert sqlite statement to entries")?;

        trace!("collected entries after {}", measure.duration());

        debug!("done getting entries after {}", measure.done());

        Ok(entries)
    }

    fn get_entry_by_id(&self, _entry_id: usize, _project: &str) -> Result<Entry, Error> {
        unimplemented!()
    }

    fn get_projects(&self) -> Result<Vec<String>, Error> {
        let mut measure = Measure::default();

        debug!("getting projects");

        let mut stmt = self
            .db_connection
            .prepare(include_str!("../resources/sqlite/get_projects.sql"))
            .context("can not prepare statement for query get_projects")?;

        trace!("preparted sql after {}", measure.duration());

        let projects = stmt
            .query_map(NO_PARAMS, |row| row.get(0))
            .context("can not convert rows to projects")?
            .filter_map(|project| match project {
                Ok(project) => Some(project),
                Err(err) => {
                    warn!("problem while getting row from sqlite: {}", err);
                    None
                }
            })
            .collect::<Vec<Option<String>>>()
            .into_iter()
            .map(|project| {
                if project.is_none() {
                    String::from("<none>")
                } else {
                    project.unwrap()
                }
            })
            .collect();

        debug!("done getting projects after {}", measure.done());

        Ok(projects)
    }

    fn update_entry(&self, _old: &Entry, _new: Entry) -> Result<(), Error> {
        unimplemented!()
    }
}

fn sqlite_statement_to_entries(mut stmt: Statement<'_>, project: &str) -> Result<Entries, Error> {
    let entries = stmt
        .query_map(&[&project], |row| {
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
        })
        .context("can not convert rows to entries")?
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
