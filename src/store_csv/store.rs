use crate::{
    entry::{
        Entries,
        Entry,
        Metadata,
        ProjectCount,
    },
    helper::confirm,
    store::Store,
    store_csv::{
        index::CsvIndex,
        vcs::VcsSettings,
    },
};
use chrono::Utc;
use failure::{
    bail,
    format_err,
    Error,
    ResultExt,
};
use fs2::FileExt;
use glob::glob;
use log::{
    debug,
    info,
    trace,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    collections::{
        BTreeSet,
        HashMap,
    },
    fs,
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

pub(crate) struct CsvStore {
    datadir: PathBuf,
    index: CsvIndex,
    settings: StoreSettings,
}

impl CsvStore {
    pub(crate) fn open<P: AsRef<Path>>(datadir: P) -> Result<Self, Error> {
        std::fs::create_dir_all(&datadir)?;

        let settings = CsvStore::get_settings(&datadir)?;

        dbg!(&settings);

        if settings.store_version != 1 {
            bail!("wrong store version")
        }

        Ok(Self {
            datadir: datadir.as_ref().to_path_buf(),
            index: CsvIndex::new(CsvStore::get_index_filename(&datadir)),
            settings,
        })
    }

    fn get_index_filename<P: AsRef<Path>>(datadir: P) -> PathBuf {
        let mut index_file = PathBuf::new();
        index_file.push(datadir);
        index_file.push("index.csv");

        index_file
    }

    fn get_settings<P: AsRef<Path>>(datadir: P) -> Result<StoreSettings, Error> {
        let path = CsvStore::settings_path(&datadir);

        if !path.exists() {
            let info = StoreSettings::default();
            let data = toml::to_string_pretty(&info)?;

            let mut file = fs::File::create(path)?;
            file.lock_exclusive()?;
            file.write_all(data.as_bytes())?;

            return Ok(info);
        }

        let mut file = fs::File::open(path)?;
        file.lock_exclusive()?;

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        let info = toml::from_slice(&data)?;

        Ok(info)
    }

    fn settings_path<P: AsRef<Path>>(datadir: P) -> PathBuf {
        let mut path = PathBuf::new();
        path.push(datadir);
        path.push(".settings.toml");

        path
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

    fn write_entry_text(&self, entry: &Entry) -> Result<(), Error> {
        let entry_folder = self.get_entry_foldername(&entry.metadata);
        fs::create_dir_all(&entry_folder).context("can not create entry folder")?;

        let entry_file = self.get_entry_filename(&entry.metadata);

        let mut file = fs::File::create(entry_file).context("can not create entry file")?;
        file.lock_exclusive()?;
        file.write(entry.text.as_bytes())
            .context("can not write entry text to file")?;

        Ok(())
    }

    fn get_entry_for_metadata(&self, metadata: Metadata) -> Result<Entry, Error> {
        let entry_file = self.get_entry_filename(&metadata);

        let mut file = fs::File::open(entry_file).context("can not open entry file")?;
        file.lock_exclusive()?;

        let mut text = String::new();
        file.read_to_string(&mut text)
            .context("can not read entry file text")?;

        Ok(Entry { metadata, text })
    }

    fn cleanup_unreferenced_entry(&self) -> Result<(), Error> {
        let glob_text = format!("{}/entries/**/*.adoc", self.datadir.to_str().unwrap());

        let store_uuids = self
            .get_latest_metadata()?
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

        if let Some(vcs) = &self.settings.vcs {
            vcs.commit(&self.datadir)?;
        }

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

        let new = Metadata {
            finished: Some(Utc::now()),
            last_change: Utc::now(),
            ..entry.metadata
        };

        trace!("new: {:#?}", new);

        self.index
            .insert_entry(&new)
            .context("can not add entry to done index")?;

        Ok(())
    }

    fn entry_done_by_uuid(&self, uuid: Uuid) -> Result<(), Error> {
        let entry = self
            .get_entry_by_uuid(&uuid)
            .context("can not get entry from uuid")?;

        let new = Metadata {
            finished: Some(Utc::now()),
            last_change: Utc::now(),
            ..entry.metadata
        };

        self.index
            .insert_entry(&new)
            .context("can not add entry to done index")?;

        Ok(())
    }

    fn entry_active_by_uuid(&self, uuid: Uuid) -> Result<(), Error> {
        let entry = self
            .get_entry_by_uuid(&uuid)
            .context("can not get entry from uuid")?;

        let new = Metadata {
            finished: None,
            last_change: Utc::now(),
            ..entry.metadata
        };

        self.index
            .insert_entry(&new)
            .context("can not add entry to active index")?;

        Ok(())
    }

    fn get_active_entries(&self, project: &str) -> Result<Entries, Error> {
        let entries = self
            .get_entries(project)?
            .into_iter()
            .filter(Entry::is_active)
            .collect();

        trace!("entries: {:#?}", entries);

        Ok(entries)
    }

    fn get_done_entries(&self, project: &str) -> Result<Entries, Error> {
        let entries = self
            .get_entries(project)?
            .into_iter()
            .filter(Entry::is_done)
            .collect();

        trace!("entries: {:#?}", entries);

        Ok(entries)
    }

    fn get_all_entries(&self) -> Result<Entries, Error> {
        let projects = self
            .get_projects()
            .context("can not get projects from store")?;

        let mut entries = BTreeSet::default();

        for project in projects {
            let project_entries = self
                .get_entries(&project)
                .context("can not get entries from old store")?;

            for entry in project_entries {
                entries.insert(entry);
            }
        }

        Ok(entries.into())
    }

    fn get_entries(&self, project: &str) -> Result<Entries, Error> {
        let metadata_entries = self
            .index
            .get_latest_metadata()
            .context("can not get metadata from active index")?;

        let raw_entries: Entries = metadata_entries
            .into_iter()
            .filter(|metadata| metadata.project == project)
            .map(|metadata| self.get_entry_for_metadata(metadata))
            .collect::<Result<BTreeSet<Entry>, Error>>()
            .context("can not get entry for metadata")?
            .into();

        trace!("raw_entries: {:#?}", raw_entries);

        let entries = raw_entries.latest_entries();

        trace!("entries: {:#?}", entries);

        Ok(entries)
    }

    fn get_entry_by_uuid(&self, uuid: &Uuid) -> Result<Entry, Error> {
        let metadata = self
            .index
            .get_latest_metadata()?
            .into_iter()
            .find(|entry| entry.uuid == *uuid)
            .ok_or_else(|| format_err!("entry not found"))?;

        let entry = self.get_entry_for_metadata(metadata)?;

        Ok(entry)
    }

    fn get_entry_by_id(&self, entry_id: usize, project: &str) -> Result<Entry, Error> {
        let entry = self
            .get_active_entries(project)
            .context("can not get project entries")?
            .entry_by_id(entry_id)
            .context("can not get entry by id")?;

        Ok(entry)
    }

    fn get_projects_count(&self) -> Result<Vec<ProjectCount>, Error> {
        let metadata = self.index.get_latest_metadata()?;

        let mut count: HashMap<String, ProjectCount> = HashMap::default();

        for entry in metadata {
            let old_count = count
                .entry(entry.project.clone())
                .or_insert_with(ProjectCount::default);

            let (active_count, done_count) = if entry.is_active() { (1, 0) } else { (0, 1) };

            *old_count += ProjectCount {
                project: entry.project,
                active_count,
                done_count,
                total_count: 1,
            }
        }

        trace!("count: {:#?}", count);

        Ok(count.into_iter().map(|(_, count)| count).collect())
    }

    fn get_projects(&self) -> Result<Vec<String>, Error> {
        let projects = self.index.get_projects().context("can not get projects")?;

        trace!("projects: {:#?}", projects);

        Ok(projects)
    }

    fn get_metadata(&self) -> Result<BTreeSet<Metadata>, Error> {
        self.index.get_metadata()
    }

    fn get_latest_metadata(&self) -> Result<Vec<Metadata>, Error> {
        self.index.get_latest_metadata()
    }

    fn add_metadata(&self, metadata: Metadata) -> Result<(), Error> {
        self.index.add_metadata(metadata)
    }

    fn run_cleanup(&self) -> Result<(), Error> {
        self.index.cleanup_duplicate_uuids()?;
        // TODO: This should remove index entries that dont have an entry file anymore.
        // self.cleanup_stale_index_entries()?;
        self.cleanup_unreferenced_entry()?;

        Ok(())
    }

    fn update_entry(&self, entry: Entry) -> Result<(), Error> {
        self.write_entry_text(&entry)
            .context("can not write entry text to file")?;

        let metadata = self.index.get_latest_metadata()?;

        if !metadata.contains(&entry.metadata) {
            self.add_metadata(entry.metadata)?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoreSettings {
    store_version: usize,
    vcs: Option<VcsSettings>,
}

impl Default for StoreSettings {
    fn default() -> Self {
        Self {
            store_version: 1,
            vcs: Some(VcsSettings::default()),
        }
    }
}
