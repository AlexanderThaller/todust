use crate::{
    entry_v2::{
        Entries,
        Entry,
        Metadata,
    },
    helper::confirm,
    store_v2::Store,
};
use chrono::Utc;
use csv::{
    Error as CsvError,
    ReaderBuilder,
    WriterBuilder,
};
use failure::{
    bail,
    Error,
    ResultExt,
};
use glob::glob;
use log::{
    debug,
    info,
    trace,
};
use std::{
    collections::{
        BTreeSet,
        HashMap,
    },
    fs::{
        self,
        OpenOptions,
    },
    io::{
        Read,
        Write,
    },
    path::{
        Path,
        PathBuf,
    },
};
use uuid::Uuid;

pub struct CsvStore {
    datadir: PathBuf,
}

impl CsvStore {
    pub fn open<P: AsRef<Path>>(datadir: P) -> Self {
        Self {
            datadir: datadir.as_ref().to_path_buf(),
        }
    }

    fn get_entry_foldername(&self, entry: &Metadata) -> PathBuf {
        let uuid = entry.uuid.to_string();
        debug!("uuid: {}", uuid);

        // Gets the first two characters of the uuid. This should never fail so the
        // unwrap is safe.
        let uuid_prefix = &uuid[..uuid.char_indices().nth(2).unwrap().0];
        debug!("uuid_prefix: {}", uuid_prefix);

        // {{ datadir }}/entries/{{ uuid_prefix }}
        let mut folder = PathBuf::new();
        folder.push(&self.datadir);
        folder.push("entries");
        folder.push(uuid_prefix);

        debug!("folder: {:?}", folder);

        folder
    }

    fn get_entry_filename(&self, entry: &Metadata) -> PathBuf {
        let entry_folder = self.get_entry_foldername(&entry);

        let mut entry_file = PathBuf::new();
        entry_file.push(entry_folder);
        entry_file.push(format!("{}.adoc", entry.uuid));

        entry_file
    }

    fn get_active_index_filename(&self) -> PathBuf {
        let mut index_file = PathBuf::new();
        index_file.push(&self.datadir);
        index_file.push("active.csv");

        index_file
    }

    fn get_done_index_filename(&self) -> PathBuf {
        let mut index_file = PathBuf::new();
        index_file.push(&self.datadir);
        index_file.push("done.csv");

        index_file
    }

    fn write_entry_text(&self, entry: &Entry) -> Result<(), Error> {
        let entry_folder = self.get_entry_foldername(&entry.metadata);
        fs::create_dir_all(&entry_folder).context("can not create entry folder")?;

        let entry_file = self.get_entry_filename(&entry.metadata);

        let mut file = fs::File::create(entry_file).context("can not create entry file")?;
        file.write(entry.text.as_bytes())
            .context("can not write entry text to file")?;

        Ok(())
    }

    fn add_entry_to_index(&self, index_path: PathBuf, entry: &Metadata) -> Result<(), Error> {
        let mut builder = WriterBuilder::new();
        // We only want to write the header if the file does not exist yet so we can
        // just append new entries to the existing file without having multiple
        // headers.
        builder.has_headers(!index_path.exists());

        let index_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&index_path)
            .context("can not open index_file")?;

        let mut writer = builder.from_writer(index_file);

        writer
            .serialize(&entry)
            .context("can not serialize entry to csv")?;

        Ok(())
    }

    fn add_entry_to_active_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_active_index_filename();
        self.add_entry_to_index(index_path, entry)
    }

    fn add_entry_to_done_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_done_index_filename();
        self.add_entry_to_index(index_path, entry)
    }

    fn remove_entry_from_index(&self, index_path: PathBuf, entry: &Metadata) -> Result<(), Error> {
        let entries = {
            let mut rdr = ReaderBuilder::new()
                .from_path(&index_path)
                .context("can not open index for reading")?;

            rdr.deserialize()
                .collect::<Result<Vec<Metadata>, CsvError>>()
                .context("can not deserialize metadata csv from index")?
                .into_iter()
                .filter(|index_entry| index_entry.uuid != entry.uuid)
                .collect::<Vec<_>>()
        };

        if entries.is_empty() {
            fs::remove_file(&index_path).context("can not remove file")?;
        } else {
            let mut wtr = WriterBuilder::new()
                .from_path(&index_path)
                .context("can not open index for writing")?;

            entries
                .iter()
                .map(|entry| wtr.serialize(entry))
                .collect::<Result<(), CsvError>>()
                .context("can not serialize entries to file")?;
        };

        Ok(())
    }

    fn remove_entry_from_active_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_active_index_filename();
        self.remove_entry_from_index(index_path, entry)
    }

    fn remove_entry_from_done_index(&self, entry: &Metadata) -> Result<(), Error> {
        let index_path = self.get_done_index_filename();
        self.remove_entry_from_index(index_path, entry)
    }

    fn get_entry_for_metadata(&self, metadata: Metadata) -> Result<Entry, Error> {
        let entry_file = self.get_entry_filename(&metadata);

        let mut file = fs::File::open(entry_file).context("can not open entry file")?;

        let mut text = String::new();
        file.read_to_string(&mut text)
            .context("can not read entry file text")?;

        Ok(Entry { metadata, text })
    }

    fn remove_entry(&self, entry: &Metadata) -> Result<(), Error> {
        if entry.finished.is_none() {
            self.remove_entry_from_active_index(entry)
                .context("can not add entry to active index")?;
        } else {
            self.remove_entry_from_done_index(entry)
                .context("can not add entry to done index")?;
        }

        Ok(())
    }

    fn get_projects_from_index(&self, index_path: PathBuf) -> Result<Vec<String>, Error> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(index_path)
            .context("can not build csv reader")?;

        let projects = rdr
            .deserialize()
            .collect::<Result<Vec<Metadata>, CsvError>>()
            .context("can not deserialize csv for active entries")?
            .into_iter()
            .map(|metadata| metadata.project)
            .collect();

        Ok(projects)
    }

    fn get_metadata_entries<P: AsRef<Path>>(&self, index_path: P) -> Result<Vec<Metadata>, Error> {
        if !index_path.as_ref().exists() {
            return Ok(Vec::default());
        }

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(index_path)
            .context("can not build csv reader")?;

        let metadata_entries = rdr
            .deserialize()
            .collect::<Result<Vec<Metadata>, CsvError>>()
            .context("can not deserialize csv for active entries")?;

        trace!("metadata_entries: {:#?}", metadata_entries);

        Ok(metadata_entries)
    }

    fn get_all_metadata_entries(&self) -> Result<Vec<Metadata>, Error> {
        let metadata_entries = {
            let mut active_metadata_entries = self
                .get_active_metadata_entries()
                .context("can not get metadata from active index")?;

            let mut done_metadata_entries = self
                .get_done_metadata_entries()
                .context("can not get metadata from active index")?;

            active_metadata_entries.append(&mut done_metadata_entries);
            active_metadata_entries
        };

        Ok(metadata_entries)
    }

    fn get_active_metadata_entries(&self) -> Result<Vec<Metadata>, Error> {
        let index_path = self.get_active_index_filename();
        let metadata_entries = self.get_metadata_entries(&index_path)?;

        Ok(metadata_entries)
    }

    fn get_done_metadata_entries(&self) -> Result<Vec<Metadata>, Error> {
        let index_path = self.get_done_index_filename();
        let metadata_entries = self.get_metadata_entries(&index_path)?;

        Ok(metadata_entries)
    }

    fn add_metadata_to_store(&self, metadata: Metadata) -> Result<(), Error> {
        match metadata.finished {
            None => self
                .add_entry_to_active_index(&metadata)
                .context("can not add entry to active index")?,
            Some(_) => self
                .add_entry_to_done_index(&metadata)
                .context("can not add entry to done index")?,
        }

        Ok(())
    }

    fn cleanup_duplicate_uuids(&self) -> Result<(), Error> {
        let mut dedup_map: HashMap<Uuid, Metadata> = HashMap::default();
        let mut seen_map: HashMap<Uuid, usize> = HashMap::default();

        let metadatas = self.get_metadata()?;
        for metadata in metadatas {
            *seen_map.entry(metadata.uuid).or_insert(0) += 1;

            match dedup_map.get(&metadata.uuid) {
                None => {
                    dedup_map.insert(metadata.uuid, metadata);
                }
                Some(dedup_metadata) => {
                    if metadata > *dedup_metadata {
                        dedup_map.insert(metadata.uuid, metadata);
                    }
                }
            };
        }

        let duplicate_entries = seen_map
            .into_iter()
            .filter(|(uuid, seen_count)| {
                debug!("seen uuid {}, {} times", uuid, seen_count);
                *seen_count != 1
            })
            .map(|(uuid, _)| uuid)
            .collect::<Vec<Uuid>>();

        trace!("dedup_map: {:#?}", dedup_map);

        for uuid in duplicate_entries {
            info!("found duplicated entries for uuid {}", uuid);

            let metadata = &dedup_map[&uuid];
            self.remove_entry_from_active_index(&metadata)?;
            self.remove_entry_from_done_index(&metadata)?;
            self.add_metadata(metadata.clone())?
        }

        Ok(())
    }

    fn cleanup_stale_index_entries(&self) -> Result<(), Error> {
        let metadatas = self.get_metadata()?;

        for metadata in metadatas {
            let filename = self.get_entry_filename(&metadata);

            if !filename.exists() {
                info!("removed stale metadata {:?}", metadata);
                self.remove_metadata(&metadata)?
            }
        }

        Ok(())
    }

    fn cleanup_unreferenced_entry(&self) -> Result<(), Error> {
        let glob_text = format!("{}/entries/**/*.adoc", self.datadir.to_str().unwrap());

        let store_uuids = self
            .get_metadata()?
            .iter()
            .map(|metadata| metadata.uuid)
            .collect::<BTreeSet<_>>();

        for entry in glob(&glob_text).context("failed to read glob pattern")? {
            if let Ok(path) = entry {
                let uuid = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .context("can not parse uuid from file name")?;

                if !store_uuids.contains(&uuid) {
                    info!("remove unreferenced entry: {:?}", path);
                    fs::remove_file(path)?;
                }

                trace!("uuid from file entry: {:?}", uuid);
            }
        }

        Ok(())
    }
}

impl Store for CsvStore {
    fn add_entry(&self, entry: Entry) -> Result<(), Error> {
        self.write_entry_text(&entry)
            .context("can not write entry text to file")?;

        self.add_metadata(entry.metadata)?;

        Ok(())
    }

    fn entry_done(&self, entry_id: usize, project: &str) -> Result<(), Error> {
        // TODO: Change this to only fetch the metadata as we dont need to touch the
        // entry text.
        let entry = self
            .get_entry_by_id(entry_id, project)
            .context("can not get entry from id")?;

        // TODO: This should be handled in main not by the store.
        let message = format!("do you want to finish this entry?:\n{}", entry.to_string());
        if !confirm(&message, false)? {
            bail!("not finishing task then")
        }

        self.remove_entry_from_active_index(&entry.metadata)
            .context("can not remove entry from active index")?;

        let new = Metadata {
            finished: Some(Utc::now()),
            last_change: Utc::now(),
            ..entry.metadata.clone()
        };

        trace!("new: {:#?}", new);

        self.add_entry_to_done_index(&new)
            .context("can not add entry to done index")?;

        Ok(())
    }

    fn get_active_count(&self, project: &str) -> Result<usize, Error> {
        let count = self
            .get_active_metadata_entries()
            .context("can not get metadata from active index")?
            .iter()
            .filter(|metadata| {
                if project == "%" {
                    true
                } else {
                    metadata.project == project
                }
            })
            .count();

        Ok(count)
    }

    fn get_active_entries(&self, project: &str) -> Result<Entries, Error> {
        let metadata_entries = self
            .get_active_metadata_entries()
            .context("can not get metadata from active index")?;

        let entries = metadata_entries
            .into_iter()
            .filter(|metadata| metadata.project == project)
            .map(|metadata| self.get_entry_for_metadata(metadata))
            .collect::<Result<BTreeSet<Entry>, Error>>()
            .context("can not get entry for metadata")?;

        trace!("entries: {:#?}", entries);

        Ok(Entries { entries })
    }

    fn get_count(&self, project: &str) -> Result<usize, Error> {
        let count = self.get_active_count(project)? + self.get_done_count(project)?;
        Ok(count)
    }

    fn get_done_count(&self, project: &str) -> Result<usize, Error> {
        let count = self
            .get_done_metadata_entries()
            .context("can not get metadata from active index")?
            .iter()
            .filter(|metadata| {
                if project == "%" {
                    true
                } else {
                    metadata.project == project
                }
            })
            .count();

        Ok(count)
    }

    fn get_entries(&self, project: &str) -> Result<Entries, Error> {
        let metadata_entries = self.get_all_metadata_entries()?;

        let entries = metadata_entries
            .into_iter()
            .filter(|metadata| metadata.project == project)
            .map(|metadata| self.get_entry_for_metadata(metadata))
            .collect::<Result<BTreeSet<Entry>, Error>>()
            .context("can not get entry for metadata")?;

        trace!("entries: {:#?}", entries);

        Ok(Entries { entries })
    }

    fn get_entry_by_id(&self, entry_id: usize, project: &str) -> Result<Entry, Error> {
        let entry = self
            .get_active_entries(project)
            .context("can not get project entries")?
            .entry_by_id(entry_id)
            .context("can not get entry by id")?;

        Ok(entry)
    }

    fn get_projects(&self) -> Result<Vec<String>, Error> {
        let mut active_projects = self
            .get_projects_from_index(self.get_active_index_filename())
            .context("can not get active projects")?;

        let mut done_projects = self
            .get_projects_from_index(self.get_active_index_filename())
            .context("can not get done projects")?;

        active_projects.append(&mut done_projects);
        active_projects.sort();
        active_projects.dedup();

        trace!("active_projects: {:#?}", active_projects);

        Ok(active_projects)
    }

    fn update_entry(&self, old: &Entry, new: Entry) -> Result<(), Error> {
        self.remove_entry(&old.metadata)?;
        self.add_entry(new)?;

        Ok(())
    }

    fn get_metadata(&self) -> Result<Vec<Metadata>, Error> {
        self.get_all_metadata_entries()
    }

    fn add_metadata(&self, metadata: Metadata) -> Result<(), Error> {
        self.add_metadata_to_store(metadata)
    }

    fn remove_metadata(&self, metadata: &Metadata) -> Result<(), Error> {
        self.remove_entry(metadata)
    }

    fn run_cleanup(&self) -> Result<(), Error> {
        self.cleanup_duplicate_uuids()?;
        self.cleanup_stale_index_entries()?;
        self.cleanup_unreferenced_entry()?;

        Ok(())
    }
}
