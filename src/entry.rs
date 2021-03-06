use crate::templating;
use anyhow::{
    bail,
    Error,
};
use chrono::{
    DateTime,
    NaiveDate,
    Utc,
};
use core::ops::AddAssign;
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fmt,
    iter::FromIterator,
    ops::Add,
};
use tera::{
    Context,
    Tera,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Ord, Eq, PartialOrd, PartialEq, Clone)]
pub(super) struct Metadata {
    pub(super) last_change: DateTime<Utc>,
    pub(super) due: Option<NaiveDate>,
    pub(super) started: DateTime<Utc>,
    pub(super) project: String,
    pub(super) finished: Option<DateTime<Utc>>,
    pub(super) uuid: Uuid,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            last_change: Utc::now(),
            project: "default".to_owned(),
            started: Utc::now(),
            finished: None,
            due: None,
            uuid: Uuid::new_v4(),
        }
    }
}

impl Metadata {
    pub(super) fn is_active(&self) -> bool {
        self.finished.is_none()
    }

    pub(super) fn is_done(&self) -> bool {
        self.finished.is_some()
    }
}

#[derive(Serialize, Deserialize, Debug, Ord, Eq, PartialOrd, PartialEq, Clone)]
pub(super) struct Entry {
    pub(super) metadata: Metadata,
    pub(super) text: String,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            metadata: Metadata::default(),
            text: String::new(),
        }
    }
}

impl Entry {
    pub(super) fn is_active(&self) -> bool {
        self.metadata.is_active()
    }

    pub(super) fn is_done(&self) -> bool {
        self.metadata.is_done()
    }

    pub(super) fn age(&self) -> ::chrono::Duration {
        Utc::now().signed_duration_since(self.metadata.started)
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line = self
            .text
            .replace("\n", " ")
            .chars()
            .take(100)
            .fold(String::new(), |acc, x| format!("{}{}", acc, x));

        write!(f, "{}", line)
    }
}

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub(super) struct Entries {
    pub(super) entries: BTreeSet<Entry>,
}

impl From<BTreeSet<Entry>> for Entries {
    fn from(entries: BTreeSet<Entry>) -> Self {
        Self { entries }
    }
}

impl From<Entry> for Entries {
    fn from(entry: Entry) -> Self {
        let mut entries = BTreeSet::new();
        entries.insert(entry);

        Self { entries }
    }
}

impl Entries {
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn get_active(self) -> Entries {
        self.into_iter().filter(Entry::is_active).collect()
    }

    pub(super) fn entry_by_id(self, id: usize) -> Result<Entry, Error> {
        let active_entries: Entries = self.get_active();

        if active_entries.len() < id {
            bail!("no active entry found with id {}", id)
        }

        let (_, entry) = active_entries.into_iter().enumerate().nth(id - 1).unwrap();

        Ok(entry)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn latest_entries(self) -> Self {
        let mut latest = BTreeMap::new();

        for entry in self.entries {
            latest.insert(entry.metadata.uuid, entry);
        }

        let entries = latest
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<BTreeSet<Entry>>();

        entries.into()
    }

    pub(super) fn into_inner(self) -> BTreeSet<Entry> {
        self.entries
    }
}

impl fmt::Display for Entries {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut active: BTreeMap<&str, BTreeSet<&Entry>> = BTreeMap::default();
        let mut done: BTreeMap<&str, BTreeSet<&Entry>> = BTreeMap::default();

        for entry in &self.entries {
            if entry.metadata.finished.is_none() {
                active
                    .entry(&entry.metadata.project)
                    .or_insert_with(BTreeSet::default)
                    .insert(entry);
            } else {
                done.entry(&entry.metadata.project)
                    .or_insert_with(BTreeSet::default)
                    .insert(entry);
            }
        }

        let mut context = Context::new();
        context.insert("active", &active);

        if !done.is_empty() {
            context.insert("done", &done);
        }

        let mut tera = Tera::default();
        tera.add_raw_template(
            "entries.asciidoc",
            include_str!("../resources/templates/entries.asciidoc"),
        )
        .expect("can not compile entries.asciidoc template");
        tera.register_filter("single_line", templating::single_line);
        tera.register_filter("lines", templating::lines);
        tera.register_filter("format_duration_since", templating::format_duration_since);
        tera.register_filter("some_or_dash", templating::some_or_dash);

        let rendered = tera
            .render("entries.asciidoc", &context)
            .expect("can not render remplate for entries");

        write!(f, "{}", rendered)
    }
}

impl FromIterator<Entry> for Entries {
    fn from_iter<I: IntoIterator<Item = Entry>>(iter: I) -> Self {
        let mut set = BTreeSet::new();
        set.extend(iter);
        Entries { entries: set }
    }
}

impl IntoIterator for Entries {
    type Item = Entry;
    type IntoIter = ::std::collections::btree_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Entries {
    type Item = &'a Entry;
    type IntoIter = ::std::collections::btree_set::Iter<'a, Entry>;

    fn into_iter(self) -> ::std::collections::btree_set::Iter<'a, Entry> {
        self.entries.iter()
    }
}

#[derive(Debug, Default, Ord, PartialOrd, Eq, PartialEq, Serialize)]
pub(super) struct ProjectCount {
    pub(super) project: String,
    pub(super) active_count: usize,
    pub(super) done_count: usize,
    pub(super) total_count: usize,
}

impl Add for ProjectCount {
    type Output = ProjectCount;

    fn add(self, other: ProjectCount) -> ProjectCount {
        Self {
            project: other.project,
            active_count: self.active_count + other.active_count,
            done_count: self.done_count + other.done_count,
            total_count: self.total_count + other.total_count,
        }
    }
}

impl AddAssign for ProjectCount {
    fn add_assign(&mut self, other: ProjectCount) {
        *self = Self {
            project: other.project,
            active_count: self.active_count + other.active_count,
            done_count: self.done_count + other.done_count,
            total_count: self.total_count + other.total_count,
        }
    }
}
