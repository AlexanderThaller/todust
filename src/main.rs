mod entry;
mod helper;
mod opt;
mod store;
mod store_csv;

use crate::{
    entry::{
        Entry,
        Metadata,
    },
    helper::{
        format_duration,
        format_timestamp,
        string_from_editor,
    },
    opt::{
        AddSubCommandOpts,
        DoneSubCommandOpts,
        DueSubCommandOpts,
        EditSubCommandOpts,
        ImportSubCommandOpts,
        MoveSubCommandOpts,
        Opt,
        PrintSubCommandOpts,
        ProjectsSubCommandOpts,
        SubCommand,
    },
    store::Store,
    store_csv::CsvStore,
};
use chrono::Utc;
use failure::{
    bail,
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
use simplelog::{
    Config,
    TermLogger,
};
use structopt::StructOpt;

fn main() {
    if let Err(err) = run() {
        error!("{}", format_err(&err));
        trace!("backtrace:\n{}", err.backtrace());

        ::std::process::exit(1);
    }
}

fn format_err(err: &Error) -> String {
    let mut causes = String::new();
    for c in err.iter_chain() {
        causes += &format!("{}: ", c);
    }

    let causes = causes.trim_start().trim_start_matches(':');

    causes.to_owned()
}

fn run() -> Result<(), Error> {
    let opt = Opt::from_args();

    // setup logging
    {
        let mut config = Config::default();
        config.time_format = Some("%+");

        if let Err(err) = TermLogger::init(opt.log_level, config) {
            eprintln!("can not initialize logger: {}", err);
            ::std::process::exit(1);
        }
    }

    trace!("opt: {:#?}", opt);

    match &opt.cmd {
        SubCommand::Add(sub_opt) => run_add(&opt, sub_opt),
        SubCommand::Cleanup => run_cleanup(&opt),
        SubCommand::Done(sub_opt) => run_done(&opt, sub_opt),
        SubCommand::Edit(sub_opt) => run_edit(&opt, sub_opt),
        SubCommand::List => run_list(&opt),
        SubCommand::Move(sub_opt) => run_move(&opt, sub_opt),
        SubCommand::Print(sub_opt) => run_print(&opt, sub_opt),
        SubCommand::Projects(sub_opt) => run_projects(&opt, sub_opt),
        SubCommand::Import(sub_opt) => run_import(&opt, sub_opt),
        SubCommand::Due(sub_opt) => run_due(&opt, sub_opt),
    }
}

fn run_add(opt: &Opt, sub_opt: &AddSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);

    let text = if let Some(opt_text) = &sub_opt.text {
        opt_text.clone()
    } else {
        string_from_editor(None).context("can not get message from editor")?
    };

    let entry = Entry {
        text,
        metadata: Metadata {
            project: opt.project.clone(),
            ..Metadata::default()
        },
    };

    store
        .add_entry(entry)
        .context("can not add entry to store")?;

    Ok(())
}

fn run_done(opt: &Opt, sub_opt: &DoneSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);
    store.entry_done(sub_opt.entry_id, &opt.project)?;

    Ok(())
}

fn run_edit(opt: &Opt, sub_opt: &EditSubCommandOpts) -> Result<(), Error> {
    if sub_opt.entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let store = CsvStore::open(&opt.datadir);

    let old_entry = store
        .get_entry_by_id(sub_opt.entry_id, &opt.project)
        .context("can not get entry")?;

    let new_text = string_from_editor(Some(&old_entry.text)).context(
        "can not edit entry with
editor",
    )?;

    let new_entry = if sub_opt.update_time {
        Entry {
            text: new_text,
            metadata: Metadata {
                started: Utc::now(),
                last_change: Utc::now(),
                ..old_entry.metadata.clone()
            },
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

fn run_list(opt: &Opt) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);

    let entries = store
        .get_active_entries(&opt.project)
        .context("can not get entries from store")?;

    if entries.is_empty() {
        println!("no active todos");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b->"ID", b->"Age", b->"Due", b->"Description"]);

    for (index, entry) in entries.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            format_duration(entry.age()),
            format_timestamp(entry.metadata.due),
            format!("{}", entry),
        ]);
    }

    table.printstd();

    Ok(())
}

fn run_move(opt: &Opt, sub_opt: &MoveSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);

    let old_entry = store
        .get_entry_by_id(sub_opt.entry_id, &opt.project)
        .context("can not get entry")?;

    let new_entry = Entry {
        text: old_entry.text.clone(),
        metadata: Metadata {
            project: sub_opt.target_project.clone(),
            last_change: Utc::now(),
            ..old_entry.metadata.clone()
        },
    };

    store
        .update_entry(&old_entry, new_entry)
        .context("can not update entry")?;

    Ok(())
}

fn run_print(opt: &Opt, sub_opt: &PrintSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);

    let project = opt.project.clone();

    match sub_opt.entry_id {
        Some(entry_id) => {
            let entry = store
                .get_entry_by_id(entry_id, &project)
                .context("can not get entry")?;
            println!("{}", entry.to_string());
        }

        None => {
            if sub_opt.no_done {
                let entries = store
                    .get_active_entries(&project)
                    .context("can not get entries from store")?;

                println!("{}", entries);
            } else {
                let entries = store
                    .get_entries(&project)
                    .context("can not get entries from store")?;

                println!("{}", entries);
            }
        }
    }

    Ok(())
}

fn run_projects(opt: &Opt, sub_opt: &ProjectsSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);

    let projects = store
        .get_projects()
        .context("can not get projects from store")?;

    let mut projects: Vec<_> = projects
        .iter()
        .map(|project| {
            let active_count = store.get_active_count(&project).ok().unwrap_or_default();

            let done_count = store.get_done_count(&project).ok().unwrap_or_default();

            let count = store.get_count(&project).ok().unwrap_or_default();

            (project, active_count, done_count, count)
        })
        .filter(|(_, active_count, ..)| sub_opt.print_inactive || active_count != &0)
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

    let active_count = store.get_active_count("%").ok().unwrap_or_default();
    let done_count = store.get_done_count("%").ok().unwrap_or_default();
    let count = store.get_count("%").ok().unwrap_or_default();
    table.add_row(row!["", "------", "----", "-----"]);
    table.add_row(row!["", b->active_count,
b->done_count, b->count]);

    table.printstd();

    Ok(())
}

fn run_cleanup(opt: &Opt) -> Result<(), Error> {
    CsvStore::open(&opt.datadir).run_cleanup()
}

fn run_import(opt: &Opt, sub_opt: &ImportSubCommandOpts) -> Result<(), Error> {
    let from_store = CsvStore::open(&sub_opt.from_path);
    let new_store = CsvStore::open(&opt.datadir);

    let projects = if sub_opt.import_all {
        from_store
            .get_projects()
            .context("can not get projects from old store")?
    } else {
        vec![opt.project.clone()]
    };

    for project in projects {
        let entries = from_store
            .get_entries(&project)
            .context("can not get entries from old store")?;

        for entry in entries {
            trace!("entry: {:#?}", entry);

            let new_entry = Entry {
                metadata: Metadata {
                    last_change: Utc::now(),
                    ..entry.metadata
                },
                ..entry
            };

            new_store
                .add_entry(new_entry)
                .context("can not add entry to new store")?;
        }
    }

    Ok(())
}

fn run_due(opt: &Opt, sub_opt: &DueSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir);

    let old_entry = store
        .get_entry_by_id(sub_opt.entry_id, &opt.project)
        .context("can not get entry")?;

    let new_entry = Entry {
        text: old_entry.text.clone(),
        metadata: Metadata {
            due: Some(sub_opt.due_date),
            last_change: Utc::now(),
            ..old_entry.metadata.clone()
        },
    };

    store
        .update_entry(&old_entry, new_entry)
        .context("can not update entry")?;

    Ok(())
}
