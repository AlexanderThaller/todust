use chrono::{
    DateTime,
    Utc,
};
use std::collections::BTreeSet;
use std::fmt;
use std::iter::FromIterator;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Ord, Eq, PartialOrd, PartialEq)]
pub struct Entry {
    pub started: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    uuid: Uuid,
    pub text: String,
}
impl Default for Entry {
    fn default() -> Self {
        Self {
            started: Utc::now(),
            uuid: Uuid::new_v4(),
            finished: None,
            text: String::new(),
        }
    }
}

impl Entry {
    pub fn with_text(self, text: String) -> Self {
        Self { text: text, ..self }
    }

    pub fn is_active(&self) -> bool {
        self.finished.is_none()
    }

    pub fn age(&self) -> ::chrono::Duration {
        Utc::now().signed_duration_since(self.started)
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let line = self.text
            .replace("\n", " ")
            .chars()
            .take(50)
            .fold(String::new(), |acc, x| format!("{}{}", acc, x));

        write!(f, "{}", line)
    }
}

pub struct Entries {
    entries: BTreeSet<Entry>,
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
                .take(50)
                .fold(String::new(), |acc, x| format!("{}{}", acc, x));

            writeln!(f, "=== {}\n", headline)?;
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

            writeln!(f, "")?;

            Ok(())
        };

        // Active entries
        writeln!(f, "== Active\n")?;
        for entry in self.entries
            .iter()
            .filter(|entry| entry.finished.is_none())
            .collect::<BTreeSet<_>>()
        {
            fmt_entry(f, entry)?;
        }

        // Done entries
        {
            let done = self.entries
                .iter()
                .filter(|entry| entry.finished.is_some())
                .collect::<BTreeSet<_>>();

            if !done.is_empty() {
                writeln!(f, "== Done\n")?;
                for entry in self.entries
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
