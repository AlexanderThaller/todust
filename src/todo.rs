use chrono::{
    DateTime,
    Utc,
};
use failure::Error;
use std::collections::BTreeSet;
use std::fmt;
use std::iter::FromIterator;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Ord, Eq, PartialOrd, PartialEq, Clone)]
pub struct Entry {
    // FIXME: Rename project_name to project.
    pub project_name: Option<String>,
    pub started: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub uuid: Uuid,
    pub text: String,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            project_name: None,
            started: Utc::now(),
            finished: None,
            uuid: Uuid::new_v4(),
            text: String::new(),
        }
    }
}

impl Entry {
    pub fn with_project(self, project_name: Option<String>) -> Self {
        Self {
            project_name,
            ..self
        }
    }

    pub fn with_text(self, text: String) -> Self {
        Self { text, ..self }
    }

    pub fn is_active(&self) -> bool {
        self.finished.is_none()
    }

    pub fn age(&self) -> ::chrono::Duration {
        Utc::now().signed_duration_since(self.started)
    }

    pub fn to_string(&self) -> String {
        format!("{}\n{}", self.started, self.text)
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let line = self
            .text
            .replace("\n", " ")
            .chars()
            .take(100)
            .fold(String::new(), |acc, x| format!("{}{}", acc, x));

        write!(f, "{}", line)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Entries {
    entries: BTreeSet<Entry>,
}

impl Entries {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn insert(&mut self, entry: Entry) -> bool {
        self.entries.insert(entry)
    }

    pub fn remove(&mut self, entry: &Entry) -> bool {
        self.entries.remove(entry)
    }

    pub fn get_active(self) -> Entries {
        self.into_iter().filter(|entry| entry.is_active()).collect()
    }

    pub fn entry_by_id(self, id: usize) -> Result<Entry, Error> {
        let active_entries: Entries = self.get_active();

        if active_entries.len() < id {
            bail!("no active entry found with id {}", id)
        }

        let (_, entry) = active_entries.into_iter().enumerate().nth(id - 1).unwrap();

        Ok(entry)
    }
}

impl fmt::Display for Entries {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let header = include_str!("../resources/header.asciidoc");
        writeln!(f, "{}", header)?;

        let fmt_entry = |f: &mut fmt::Formatter, entry: &Entry| -> fmt::Result {
            let headline = entry
                .text
                .replace("\n", " ")
                .chars()
                .take(100)
                .fold(String::new(), |acc, x| format!("{}{}", acc, x));

            writeln!(f, "=== {}\n", headline)?;
            if entry.project_name.is_some() {
                writeln!(f, "Project:: {}", entry.project_name.as_ref().unwrap())?;
            }
            writeln!(f, "UUID:: {}", entry.uuid)?;
            writeln!(f, "Started:: {}", entry.started)?;

            if entry.finished.is_some() {
                writeln!(f, "Finished:: {}", entry.finished.unwrap())?;
            }

            let lines = entry.text.lines();

            writeln!(f, "\n====\n")?;
            let mut is_codeblock = false;
            for line in lines {
                if line == "----" {
                    is_codeblock = !is_codeblock;
                }

                if is_codeblock {
                    writeln!(f, "{}", line)?;
                } else {
                    writeln!(f, "{}\n", line)?;
                }
            }
            writeln!(f, "====")?;

            writeln!(f)?;

            Ok(())
        };

        // Active entries
        writeln!(f, "== Active\n")?;
        for entry in self
            .entries
            .iter()
            .filter(|entry| entry.finished.is_none())
            .collect::<BTreeSet<_>>()
        {
            fmt_entry(f, entry)?;
        }

        // Done entries
        {
            let done = self
                .entries
                .iter()
                .filter(|entry| entry.finished.is_some())
                .collect::<BTreeSet<_>>();

            if !done.is_empty() {
                writeln!(f, "== Done\n")?;
                for entry in self
                    .entries
                    .iter()
                    .filter(|entry| entry.finished.is_some())
                    .collect::<BTreeSet<_>>()
                {
                    fmt_entry(f, entry)?;
                }
            }
        }

        Ok(())
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
