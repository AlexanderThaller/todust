#[macro_use]
extern crate clap;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate simplelog;

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

mod helper;
mod store;
mod todo;

use chrono::Utc;
use clap::ArgMatches;
use failure::{
    Context,
    Error,
    ResultExt,
};
use helper::{
    format_duration,
    string_from_editor,
};
use prettytable::{
    format,
    Table,
};
use std::path::PathBuf;
use todo::Entry;

fn main() {
    if let Err(err) = run() {
        let mut causes = String::new();
        for cause in err.causes() {
            causes.push_str(format!(": {}", cause).as_str())
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

        if let Err(err) = TermLogger::init(
            value_t!(matches, "log_level", LevelFilter)?,
            Config::default(),
        ) {
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
        _ => unreachable!(),
    }
}

fn run_add(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let store = store::Store::default().with_datafile_path(datafile_path);

    let entry = Entry::default()
        .with_text(string_from_editor(None).context("can not get message from editor")?);

    store
        .add_entry(entry)
        .context("can not add entry to store")?;

    Ok(())
}

fn run_print(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let no_done = matches.is_present("no_done");
    let entry_id = matches.value_of("entry_id");

    let store = store::Store::default().with_datafile_path(datafile_path);
    let entries = store
        .get_entries()
        .context("can not get entries from store")?;

    if entry_id.is_none() {
        if no_done {
            println!("{}", entries.get_active());
        } else {
            println!("{}", entries);
        }

        return Ok(());
    }

    let entry_id = entry_id
        .unwrap()
        .parse::<usize>()
        .context("can not parse entry_id")?;

    let entry = entries.entry_by_id(entry_id).context("can not get entry")?;
    println!("{}", entry.to_string());

    Ok(())
}

fn run_list(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let store = store::Store::default().with_datafile_path(datafile_path);
    let entries = store
        .get_entries()
        .context("can not get entries from store")?
        .get_active();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(row![b -> "ID", b -> "Age", b -> "Description"]);
    for (index, entry) in entries.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            format_duration(entry.age()),
            format!("{}", entry)
        ]);
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

    let store = store::Store::default().with_datafile_path(datafile_path);

    store.entry_done(entry_id)?;

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

    let store = store::Store::default().with_datafile_path(datafile_path);
    let entries = store
        .get_entries()
        .context("can not get entries from store")?;

    let old_entry = entries.entry_by_id(entry_id).context("can not get entry")?;

    let new_text =
        string_from_editor(Some(&old_entry.text)).context("can not edit entry with editor")?;

    let new_entry = if update_time {
        Entry {
            text: new_text,
            started: Utc::now(),
            ..old_entry
        }
    } else {
        Entry {
            text: new_text,
            ..old_entry
        }
    };

    store
        .update_entry(&old_entry, new_entry)
        .context("can not update entry")?;

    Ok(())
}
