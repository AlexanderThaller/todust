#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

extern crate chrono;
extern crate uuid;

extern crate csv;
extern crate serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate text_io;

extern crate tempdir;

mod todo;

use chrono::Duration;
use chrono::Utc;
use clap::ArgMatches;
use failure::{
    Context,
    Error,
    ResultExt,
};
use prettytable::{
    format,
    Table,
};
use std::fs::File;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tempdir::TempDir;
use todo::{
    Entries,
    Entry,
};

fn main() {
    if let Err(e) = run() {
        for cause in e.causes() {
            eprintln!("{}", cause);
        }

        trace!("backtrace:\n{}", e.backtrace());

        ::std::process::exit(1);
    }
}

fn run() -> Result<(), Error> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml)
        .name(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .author(crate_authors!())
        .get_matches();

    match matches.subcommand_name() {
        Some("add") => run_add(matches
            .subcommand_matches("add")
            .ok_or_else(|| Context::new("can not get subcommand matches for add"))?),
        Some("print") => run_print(matches
            .subcommand_matches("print")
            .ok_or_else(|| Context::new("can not get subcommand matches for print"))?),
        Some("list") => run_list(matches
            .subcommand_matches("list")
            .ok_or_else(|| Context::new("can not get subcommand matches for list"))?),
        Some("done") => run_done(matches
            .subcommand_matches("done")
            .ok_or_else(|| Context::new("can not get subcommand matches for done"))?),
        Some("edit") => run_edit(matches
            .subcommand_matches("edit")
            .ok_or_else(|| Context::new("can not get subcommand matches for edit"))?),
        _ => unreachable!(),
    }
}

fn run_add(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let entry = Entry::default().with_text(string_from_editor(None).context("can not get message from editor")?);

    let (file, new_file) = match OpenOptions::new().append(true).open(&datafile_path) {
        Ok(file) => (file, false),
        Err(_) => (
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&datafile_path)
                .context("can not open data file for writing")?,
            true,
        ),
    };

    let mut wtr = csv::WriterBuilder::new().has_headers(new_file).from_writer(file);

    wtr.serialize(entry).context("can not serialize entry to csv")?;

    wtr.flush().context("can not flush csv writer")?;

    Ok(())
}

fn run_print(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let no_done = matches.is_present("no_done");

    let entry_id = matches.value_of("entry_id");

    let mut rdr = csv::ReaderBuilder::new()
        .from_path(&datafile_path)
        .context("can not create entry reader")?;

    let entries: Entries = rdr.deserialize().filter(|result| result.is_ok()).map(|result| result.unwrap()).collect();

    if entry_id.is_none() {
        if no_done {
            let entries: Entries = entries.into_iter().filter(|entry| entry.is_active()).collect();
            println!("{}", entries);
        } else {
            println!("{}", entries);
        }

        return Ok(());
    }

    let entry_id = entry_id.unwrap().parse::<usize>().context("can not parse entry_id")?;

    let active_entries: Entries = entries.clone().into_iter().filter(|entry| entry.is_active()).collect();

    if active_entries.len() < entry_id {
        bail!("no active entry found with id {}", entry_id)
    }

    let (_, entry) = active_entries.into_iter().enumerate().nth(entry_id - 1).unwrap();

    println!("{}", entry.to_string());

    Ok(())
}

fn run_list(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let mut rdr = csv::ReaderBuilder::new()
        .from_path(&datafile_path)
        .context("can not create entry reader")?;

    let entries: Entries = rdr
        .deserialize()
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap())
        .collect::<Entries>()
        .into_iter()
        .filter(|entry| entry.is_active())
        .collect();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(row![b -> "ID", b -> "Age", b -> "Description"]);
    for (index, entry) in entries.into_iter().enumerate() {
        table.add_row(row![index + 1, format_duration(entry.age()), format!("{}", entry)]);
    }

    table.printstd();

    Ok(())
}

fn run_done(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let entry_id = value_t!(matches, "entry_id", usize).context("can not get entry_id from args")?;

    if entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let mut rdr = csv::ReaderBuilder::new()
        .from_path(&datafile_path)
        .context("can not create entry reader")?;

    let mut entries: Entries = rdr.deserialize().filter(|result| result.is_ok()).map(|result| result.unwrap()).collect();

    let active_entries: Entries = entries.clone().into_iter().filter(|entry| entry.is_active()).collect();

    trace!("active_entries: {}, entry_id: {}", active_entries.len(), entry_id);

    if active_entries.len() < entry_id {
        bail!("no active entry found with id {}", entry_id)
    }

    let (_, entry) = active_entries.into_iter().enumerate().nth(entry_id - 1).unwrap();

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

    let tmpdir = TempDir::new("todust_tmp").unwrap();
    let tmppath = tmpdir.path().join("data.csv");

    {
        let mut wtr = csv::Writer::from_path(&tmppath).context("can not open tmpfile for serializing")?;

        for entry in entries {
            wtr.serialize(entry).context("can not serialize entry")?;
        }
    }

    ::std::fs::copy(tmppath, datafile_path).context("can not move new datafile to datafile_path")?;

    Ok(())
}

fn run_edit(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let entry_id = value_t!(matches, "entry_id", usize).context("can not get entry_id from args")?;

    let update_time = matches.is_present("update_time");

    if entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let mut rdr = csv::ReaderBuilder::new()
        .from_path(&datafile_path)
        .context("can not create entry reader")?;

    let mut entries: Entries = rdr.deserialize().filter(|result| result.is_ok()).map(|result| result.unwrap()).collect();

    let active_entries: Entries = entries.clone().into_iter().filter(|entry| entry.is_active()).collect();

    trace!("active_entries: {}, entry_id: {}", active_entries.len(), entry_id);

    if active_entries.len() < entry_id {
        bail!("no active entry found with id {}", entry_id)
    }

    let (_, entry) = active_entries.into_iter().enumerate().nth(entry_id - 1).unwrap();

    let new_text = string_from_editor(Some(&entry.text)).context("can not edit entry with editor")?;

    entries.remove(&entry);

    let entry = if update_time {
        Entry {
            text: new_text,
            started: Utc::now(),
            ..entry
        }
    } else {
        Entry { text: new_text, ..entry }
    };

    entries.insert(entry);

    let tmpdir = TempDir::new("todust_tmp").unwrap();
    let tmppath = tmpdir.path().join("data.csv");

    {
        let mut wtr = csv::Writer::from_path(&tmppath).context("can not open tmpfile for serializing")?;

        for entry in entries {
            wtr.serialize(entry).context("can not serialize entry")?;
        }
    }

    ::std::fs::copy(tmppath, datafile_path).context("can not move new datafile to datafile_path")?;

    Ok(())
}

fn format_duration(duration: Duration) -> String {
    if duration < Duration::minutes(1) {
        return format!("{}s", duration.num_seconds());
    }

    if duration < Duration::hours(1) {
        return format!("{}m", duration.num_minutes());
    }

    if duration < Duration::hours(24) {
        return format!("{}h", duration.num_hours());
    }

    format!("{}d", duration.num_days())
}

pub fn string_from_editor(prepoluate: Option<&str>) -> Result<String, Error> {
    use std::env;
    use std::io::{
        Read,
        Write,
    };
    use std::process::Command;

    let tmpdir = TempDir::new("todust_tmp").unwrap();
    let tmppath = tmpdir.path().join("todo.asciidoc");
    let editor = {
        match env::var("VISUAL") {
            Ok(editor) => editor,
            Err(_) => match env::var("EDITOR") {
                Ok(editor) => editor,
                Err(_) => bail!("not editor set. either set $VISUAL OR $EDITOR environment variable"),
            },
        }
    };

    if let Some(content) = prepoluate {
        let mut file = File::create(tmppath.display().to_string()).context("can not open tmp editor file to prepoluate with string")?;

        file.write_all(content.as_bytes()).context("can not prepoluate editor tmp file")?;
    }

    let mut editor_command = Command::new(editor);
    editor_command.arg(tmppath.display().to_string());

    editor_command
        .spawn()
        .context("couldn not launch editor")?
        .wait()
        .context("problem while running editor")?;

    let mut string = String::new();
    let mut file = File::open(tmppath).context("can not open tmppath for reading")?;

    file.read_to_string(&mut string).context("can not read tmpfile to string")?;

    Ok(string)
}

fn confirm(message: &str, default: bool) -> Result<bool, Error> {
    let default_text = if default { "Y/n" } else { "N/y" };

    println!("{}\n({}): ", message, default_text);
    let input: String = read!("{}\n");

    match input.trim().to_uppercase().as_str() {
        "Y" | "YES" => Ok(true),
        "N" | "NO" => Ok(false),
        _ => bail!("do not know what to do with {}", input),
    }
}
