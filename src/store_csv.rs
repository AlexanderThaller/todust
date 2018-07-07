use chrono::Utc;
use csv::{
    Error as CsvError,
    ReaderBuilder,
    Writer,
    WriterBuilder,
};
use failure::{
    Error,
    ResultExt,
};
use helper::confirm;
use std::fs::OpenOptions;
use std::path::PathBuf;
use store::Store;
use tempfile::tempdir;
use todo::{
    Entries,
    Entry,
};

#[derive(Default)]
pub struct CSVStore {
    datafile_path: PathBuf,
}

impl Store for CSVStore {
    fn with_datafile_path(self, datafile_path: PathBuf) -> Self {
        Self { datafile_path }
    }

    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        let (file, new_file) = match OpenOptions::new().append(true).open(&self.datafile_path) {
            Ok(file) => (file, false),
            Err(_) => (
                OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&self.datafile_path)
                    .context("can not open data file for writing")?,
                true,
            ),
        };

        let mut wtr = WriterBuilder::new().has_headers(new_file).from_writer(file);

        wtr.serialize(entry)
            .context("can not serialize entry to csv")?;

        wtr.flush().context("can not flush csv writer")?;

        Ok(())
    }

    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error> {
        let mut entries = self.get_entries()?;

        entries.remove(old);
        entries.insert(new);

        let tmpdir = tempdir().context("can not create tempdir")?;
        let tmppath = tmpdir.path().join("data.csv");

        {
            let mut wtr =
                Writer::from_path(&tmppath).context("can not open tmpfile for serializing")?;

            for entry in entries {
                wtr.serialize(entry).context("can not serialize entry")?;
            }
        }

        ::std::fs::copy(tmppath, &self.datafile_path)
            .context("can not move new datafile to datafile_path")?;

        Ok(())
    }

    fn get_entries(&self) -> Result<Entries, Error> {
        let mut rdr = ReaderBuilder::new()
            .from_path(&self.datafile_path)
            .context("can not create entry reader")?;

        let entries = rdr
            .deserialize()
            .collect::<Result<Entries, CsvError>>()
            .context("can not deserialize csv entries")?;

        Ok(entries)
    }

    fn entry_done(&self, entry_id: usize) -> Result<(), Error> {
        if entry_id < 1 {
            bail!("entry id can not be smaller than 1")
        }

        let mut rdr = ReaderBuilder::new()
            .from_path(&self.datafile_path)
            .context("can not create entry reader")?;

        let mut entries: Entries = rdr
            .deserialize()
            .filter(|result| result.is_ok())
            .map(|result| result.unwrap())
            .collect();

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

        entries.remove(&entry);

        let entry = Entry {
            finished: Some(Utc::now()),
            ..entry
        };

        entries.insert(entry);

        let tmpdir = tempdir().unwrap();
        let tmppath = tmpdir.path().join("data.csv");

        {
            let mut wtr =
                Writer::from_path(&tmppath).context("can not open tmpfile for serializing")?;

            for entry in entries {
                wtr.serialize(entry).context("can not serialize entry")?;
            }
        }

        ::std::fs::copy(tmppath, &self.datafile_path)
            .context("can not move new datafile to datafile_path")?;

        Ok(())
    }
}
