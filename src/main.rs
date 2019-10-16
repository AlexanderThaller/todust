mod entry;
mod helper;
mod opt;
mod store;
mod store_csv;

use crate::{
    entry::{
        Entries,
        Entry,
        Metadata,
        ProjectCount,
    },
    helper::{
        format_duration,
        format_timestamp,
        string_from_editor,
    },
    opt::*,
    store::Store,
    store_csv::{
        CsvIndex,
        CsvStore,
    },
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
use std::io::{
    self,
    Write,
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

    let causes = causes.trim().trim_matches(':');

    causes.to_owned()
}

fn run() -> Result<(), Error> {
    let opt = Opt::from_args();

    // setup logging
    {
        let config = simplelog::ConfigBuilder::new().build();

        if let Err(err) =
            { simplelog::TermLogger::init(opt.log_level, config, simplelog::TerminalMode::Stderr) }
        {
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
        SubCommand::MergeIndexFiles(sub_opt) => run_merge_index_files(&opt, sub_opt),
    }
}

fn run_add(opt: &Opt, sub_opt: &AddSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir)?;

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
    let store = CsvStore::open(&opt.datadir)?;
    store.entry_done(sub_opt.entry_id, &opt.project)?;

    Ok(())
}

fn run_edit(opt: &Opt, sub_opt: &EditSubCommandOpts) -> Result<(), Error> {
    if sub_opt.entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let store = CsvStore::open(&opt.datadir)?;

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
                ..old_entry.metadata
            },
        }
    } else {
        Entry {
            text: new_text,
            ..old_entry
        }
    };

    store
        .update_entry(new_entry)
        .context("can not update entry")?;

    Ok(())
}

fn run_list(opt: &Opt) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir)?;

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
    let store = CsvStore::open(&opt.datadir)?;

    let old_entry = store
        .get_entry_by_id(sub_opt.entry_id, &opt.project)
        .context("can not get entry")?;

    let new_entry = Entry {
        text: old_entry.text.clone(),
        metadata: Metadata {
            project: sub_opt.target_project.clone(),
            last_change: Utc::now(),
            ..old_entry.metadata
        },
    };

    store.add_entry(new_entry).context("can not add entry")?;

    Ok(())
}

fn run_print(opt: &Opt, sub_opt: &PrintSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir)?;

    let project = opt.project.clone();

    match sub_opt.entry_id {
        Some(entry_id) => {
            let entry = store
                .get_entry_by_id(entry_id, &project)
                .context("can not get entry")?;

            let entries: Entries = entry.into();

            println!("{}", entries);
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
    if sub_opt.simple {
        run_projects_simple(opt, sub_opt)
    } else {
        run_projects_normal(opt, sub_opt)
    }
}

fn run_projects_normal(opt: &Opt, sub_opt: &ProjectsSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir)?;

    let mut projects_count = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .filter(|entry| entry.active_count != 0 || sub_opt.print_inactive)
        .collect::<Vec<_>>();

    projects_count.sort();

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![b->"Project", b->"Active", b->"Done", b->"Total"]);

    for entry in &projects_count {
        trace!("entry written to table: {:#?}", entry);

        table.add_row(row![
            entry.project,
            entry.active_count,
            entry.done_count,
            entry.total_count
        ]);
    }

    if !projects_count.is_empty() {
        table.add_row(row!["", "------", "----", "-----"]);
    }

    let total = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .fold(ProjectCount::default(), |acc, x| acc + x);

    table.add_row(row![b->"Total", b->total.active_count,
b->total.done_count, b->total.total_count]);

    table.printstd();

    Ok(())
}

fn run_projects_simple(opt: &Opt, sub_opt: &ProjectsSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir)?;

    let mut projects_count = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .filter(|entry| entry.active_count != 0 || sub_opt.print_inactive)
        .collect::<Vec<_>>();

    projects_count.sort();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for entry in projects_count {
        handle.write_all(entry.project.as_bytes())?;
        handle.write_all(b"\n")?;
    }

    Ok(())
}

fn run_cleanup(opt: &Opt) -> Result<(), Error> {
    CsvStore::open(&opt.datadir)?.run_cleanup()
}

fn run_import(opt: &Opt, sub_opt: &ImportSubCommandOpts) -> Result<(), Error> {
    let from_store = CsvStore::open(&sub_opt.from_path)?;
    let new_store = CsvStore::open(&opt.datadir)?;

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
    let store = CsvStore::open(&opt.datadir)?;

    let old_entry = store
        .get_entry_by_id(sub_opt.entry_id, &opt.project)
        .context("can not get entry")?;

    let new_entry = Entry {
        text: old_entry.text,
        metadata: Metadata {
            due: Some(sub_opt.due_date),
            last_change: Utc::now(),
            ..old_entry.metadata
        },
    };

    store.add_entry(new_entry).context("can not add entry")?;

    Ok(())
}

fn run_merge_index_files(_opt: &Opt, sub_opt: &MergeIndexFilesSubCommandOpts) -> Result<(), Error> {
    if sub_opt.output.exists() {
        if sub_opt.force {
            std::fs::remove_file(&sub_opt.output).context("can not remove existing output file")?;
        } else {
            bail!("will not overwrite existing output file")
        }
    }

    let first_store = CsvIndex::new(&sub_opt.input_first);
    let second_store = CsvIndex::new(&sub_opt.input_second);
    let output_store = CsvIndex::new(&sub_opt.output);

    let mut first_entries = first_store.get_metadata_entries()?;
    let mut second_entries = second_store.get_metadata_entries()?;

    let mut merged = std::collections::BTreeSet::default();
    merged.append(&mut first_entries);
    merged.append(&mut second_entries);

    for entry in merged {
        output_store.add_metadata_to_store(entry)?;
    }

    Ok(())
}
