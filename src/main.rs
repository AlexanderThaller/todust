mod helper;
mod measure;
mod store;
mod store_csv;
mod store_sqlite;
mod todo;

use crate::{
    helper::{
        format_duration,
        string_from_editor,
    },
    store::Store,
    store_csv::CsvStore,
    store_sqlite::SqliteStore,
    todo::Entry,
};
use chrono::Utc;
use clap::{
    crate_authors,
    crate_description,
    crate_name,
    crate_version,
    load_yaml,
    value_t,
    ArgMatches,
};
use failure::{
    bail,
    Context,
    Error,
    ResultExt,
};
use log::{
    error,
    trace,
};
use prettytable::{
    cell,
    format,
    row,
    Table,
};
use std::path::PathBuf;

fn main() {
    if let Err(err) = run() {
        let mut causes = String::new();
        for c in err.iter_chain() {
            causes += &format!(": {}", c);
        }

        error!("{}", causes);
        trace!("backtrace:\n{}", err.backtrace());

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

    // setup logging
    {
        use simplelog::*;

        let mut config = Config::default();
        config.time_format = Some("%+");

        if let Err(err) = TermLogger::init(value_t!(matches, "log_level", LevelFilter)?, config) {
            eprintln!("can not initialize logger: {}", err);
            ::std::process::exit(1);
        }
    }

    match matches.subcommand_name() {
        Some("add") => run_add(
            matches
                .subcommand_matches("add")
                .ok_or_else(|| Context::new("can not get subcommand matches for add"))?,
        ),
        Some("print") => run_print(
            matches
                .subcommand_matches("print")
                .ok_or_else(|| Context::new("can not get subcommand matches for print"))?,
        ),
        Some("list") => run_list(
            matches
                .subcommand_matches("list")
                .ok_or_else(|| Context::new("can not get subcommand matches for list"))?,
        ),
        Some("done") => run_done(
            matches
                .subcommand_matches("done")
                .ok_or_else(|| Context::new("can not get subcommand matches for done"))?,
        ),
        Some("edit") => run_edit(
            matches
                .subcommand_matches("edit")
                .ok_or_else(|| Context::new("can not get subcommand matches for edit"))?,
        ),
        Some("migrate") => run_migrate(
            matches
                .subcommand_matches("migrate")
                .ok_or_else(|| Context::new("can not get subcommand matches for migrate"))?,
        ),
        Some("projects") => run_projects(
            matches
                .subcommand_matches("projects")
                .ok_or_else(|| Context::new("can not get subcommand matches for projects"))?,
        ),
        Some("move") => run_move(
            matches
                .subcommand_matches("move")
                .ok_or_else(|| Context::new("can not get subcommand matches for move"))?,
        ),
        _ => unreachable!(),
    }
}

fn run_add(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    let entry = Entry::default()
        .with_text(string_from_editor(None).context("can not get message from editor")?)
        .with_project(matches.value_of("project").map(str::to_string));

    store
        .add_entry(entry)
        .context("can not add entry to store")?;

    Ok(())
}

fn run_print(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let project = matches.value_of("project");

    let no_done = matches.is_present("no_done");
    let entry_id = matches.value_of("entry_id");

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    if entry_id.is_none() {
        if no_done {
            let entries = store
                .get_active_entries(project)
                .context("can not get entries from store")?;

            println!("{}", entries);
        } else {
            let entries = store
                .get_entries(project)
                .context("can not get entries from store")?;

            println!("{}", entries);
        }

        return Ok(());
    }

    let entry_id = entry_id
        .unwrap()
        .parse::<usize>()
        .context("can not parse entry_id")?;

    let entry = store
        .get_entry_by_id(entry_id, project)
        .context("can not get entry")?;
    println!("{}", entry.to_string());

    Ok(())
}

fn run_list(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let project = matches.value_of("project");

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    let entries = store
        .get_active_entries(project)
        .context("can not get entries from store")?;

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b->"ID", b->"Age", b->"Description"]);

    for (index, entry) in entries.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            format_duration(entry.age()),
            format!("{}", entry),
        ]);
    }

    table.printstd();

    Ok(())
}

fn run_done(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let project = matches.value_of("project");

    let entry_id =
        value_t!(matches, "entry_id", usize).context("can not get entry_id from args")?;

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    store.entry_done(entry_id, project)?;

    Ok(())
}

fn run_edit(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let project = matches.value_of("project");

    let entry_id =
        value_t!(matches, "entry_id", usize).context("can not get entry_id from args")?;

    let update_time = matches.is_present("update_time");

    if entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    let old_entry = store
        .get_entry_by_id(entry_id, project)
        .context("can not get entry")?;

    let new_text =
        string_from_editor(Some(&old_entry.text)).context("can not edit entry with editor")?;

    let new_entry = if update_time {
        Entry {
            text: new_text,
            started: Utc::now(),
            ..old_entry.clone()
        }
    } else {
        Entry {
            text: new_text,
            ..old_entry.clone()
        }
    };

    store
        .update_entry(&old_entry, new_entry)
        .context("can not update entry")?;

    Ok(())
}

fn run_migrate(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let project = matches.value_of("project");

    let from_path: PathBuf = matches
        .value_of("from_path")
        .ok_or_else(|| Context::new("can not get from_path from args"))?
        .into();

    let old_store = CsvStore::default().with_datafile_path(from_path);
    let new_store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    let entries = old_store
        .get_entries(project)
        .context("can not get entries from old store")?;

    for entry in entries {
        trace!("entry: {:#?}", entry);

        new_store
            .add_entry(entry)
            .context("can not add entry to new store")?;
    }

    Ok(())
}

fn run_projects(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let print_inactive = matches.is_present("print_inactive");

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    let projects = store
        .get_projects()
        .context("can not get projects from store")?;

    let mut projects: Vec<_> = projects
        .iter()
        .map(|project| {
            let active_count = store
                .get_active_count(Some(&project))
                .ok()
                .unwrap_or_default();

            let done_count = store
                .get_done_count(Some(&project))
                .ok()
                .unwrap_or_default();

            let count = store.get_count(Some(&project)).ok().unwrap_or_default();

            (project, active_count, done_count, count)
        })
        .filter(|(_, active_count, ..)| print_inactive || active_count != &0)
        .collect();

    projects.sort();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b->"Project", b->"Active", b->"Done", b->"Total"]);

    for entry in projects {
        let project = entry.0;
        let active_count = entry.1;
        let done_count = entry.2;
        let count = entry.3;

        table.add_row(row![project, active_count, done_count, count]);
    }

    let active_count = store.get_active_count(Some("%")).ok().unwrap_or_default();
    let done_count = store.get_done_count(Some("%")).ok().unwrap_or_default();
    let count = store.get_count(Some("%")).ok().unwrap_or_default();
    table.add_row(row!["", "------", "----", "-----"]);
    table.add_row(row!["", b->active_count, b->done_count, b->count]);

    table.printstd();

    Ok(())
}

fn run_move(matches: &ArgMatches<'_>) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let project = matches.value_of("project");

    let entry_id =
        value_t!(matches, "entry_id", usize).context("can not get entry_id from args")?;

    let target_project = matches.value_of("target_project").map(str::to_string);

    let store = SqliteStore::default()
        .with_datafile_path(datafile_path)
        .open()?;

    let old_entry = store
        .get_entry_by_id(entry_id, project)
        .context("can not get entry")?;

    let new_entry = Entry {
        project_name: target_project,
        ..old_entry.clone()
    };

    store
        .update_entry(&old_entry, new_entry)
        .context("can not update entry")?;

    Ok(())
}
