mod entry;
mod helper;
mod opt;
mod store;
mod store_csv;
mod templating;
mod webservice;

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

        if let Err(err) = { simplelog::SimpleLogger::init(opt.log_level, config) } {
            eprintln!("can not initialize logger: {}", err);
            ::std::process::exit(1);
        }
    }

    trace!("opt: {:#?}", opt);

    match opt.cmd {
        SubCommand::Add(sub_opt) => run_add(sub_opt),
        SubCommand::Cleanup(sub_opt) => run_cleanup(sub_opt),
        SubCommand::Completion(sub_opt) => run_completion(sub_opt),
        SubCommand::Done(sub_opt) => run_done(sub_opt),
        SubCommand::Due(sub_opt) => run_due(sub_opt),
        SubCommand::Edit(sub_opt) => run_edit(sub_opt),
        SubCommand::List(sub_opt) => run_list(sub_opt),
        SubCommand::Move(sub_opt) => run_move(sub_opt),
        SubCommand::Print(sub_opt) => run_print(sub_opt),
        SubCommand::Projects(sub_opt) => run_projects(sub_opt),
        SubCommand::Web(sub_opt) => run_web(sub_opt),
    }
}

fn run_add(opt: AddSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let text = if let Some(opt_text) = &opt.text {
        opt_text.clone()
    } else {
        string_from_editor(None).context("can not get message from editor")?
    };

    let entry = Entry {
        text,
        metadata: Metadata {
            project: opt.project_opt.project,
            ..Metadata::default()
        },
    };

    store
        .add_entry(entry)
        .context("can not add entry to store")?;

    Ok(())
}

fn run_cleanup(opt: CleanupSubCommandOpts) -> Result<(), Error> {
    CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?.run_cleanup()
}

fn run_completion(opt: CompletionSubCommandOpts) -> Result<(), Error> {
    std::fs::create_dir_all(&opt.directory)?;
    Opt::clap().gen_completions(env!("CARGO_PKG_NAME"), opt.shell, opt.directory);

    Ok(())
}

fn run_done(opt: DoneSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;
    store.entry_done(opt.entry_id, &opt.project_opt.project)?;

    Ok(())
}

fn run_edit(opt: EditSubCommandOpts) -> Result<(), Error> {
    if opt.entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let old_entry = store
        .get_entry_by_id(opt.entry_id, &opt.project_opt.project)
        .context("can not get entry")?;

    let new_text = string_from_editor(Some(&old_entry.text)).context(
        "can not edit entry with
editor",
    )?;

    let new_entry = if opt.update_time {
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

fn run_list(opt: ListSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let entries = store
        .get_active_entries(&opt.project_opt.project)
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

fn run_move(opt: MoveSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let old_entry = store
        .get_entry_by_id(opt.entry_id, &opt.project_opt.project)
        .context("can not get entry")?;

    let new_entry = Entry {
        text: old_entry.text.clone(),
        metadata: Metadata {
            project: opt.target_project,
            last_change: Utc::now(),
            ..old_entry.metadata
        },
    };

    store.add_entry(new_entry).context("can not add entry")?;

    Ok(())
}

fn run_print(opt: PrintSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let project = opt.project_opt.project;

    match opt.entry_id {
        Some(entry_id) => {
            let entry = store
                .get_entry_by_id(entry_id, &project)
                .context("can not get entry")?;

            let entries: Entries = entry.into();

            println!("{}", entries);
        }

        None => {
            if opt.no_done {
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

fn run_projects(opt: ProjectsSubCommandOpts) -> Result<(), Error> {
    if opt.simple {
        run_projects_simple(opt)
    } else {
        run_projects_normal(opt)
    }
}

fn run_projects_simple(opt: ProjectsSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let mut projects_count = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .filter(|entry| entry.active_count != 0 || opt.print_inactive)
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

fn run_projects_normal(opt: ProjectsSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let mut projects_count = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .filter(|entry| entry.active_count != 0 || opt.print_inactive)
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

fn run_due(opt: DueSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    let old_entry = store
        .get_entry_by_id(opt.entry_id, &opt.project_opt.project)
        .context("can not get entry")?;

    let new_entry = Entry {
        text: old_entry.text,
        metadata: Metadata {
            due: Some(opt.due_date),
            last_change: Utc::now(),
            ..old_entry.metadata
        },
    };

    store.add_entry(new_entry).context("can not add entry")?;

    Ok(())
}

fn run_web(opt: WebSubCommandOpts) -> Result<(), Error> {
    let store = CsvStore::open(&opt.datadir_opt.datadir, &opt.datadir_opt.identifier)?;

    crate::webservice::WebService::open(store)?.run(opt.binding)
}
