mod config;
mod entry;
mod helper;
mod opt;
mod store;
mod templating;
mod webservice;

use crate::{
    config::Config,
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
};
use anyhow::{
    bail,
    Context,
    Error,
};
use chrono::Utc;
use comfy_table::{
    Attribute,
    Cell,
    Table,
};
use log::{
    error,
    trace,
};
use std::io::{
    self,
    Write,
};
use structopt::StructOpt;

#[async_std::main]
async fn main() {
    if let Err(err) = run().await {
        error!("{}", err)
    }
}

async fn run() -> Result<(), Error> {
    let opt = Opt::from_args();

    // setup logging
    if matches!(opt.cmd, SubCommand::Web(_)) {
        use tide::log::LevelFilter;

        let tide_log_level = match opt.log_level {
            simplelog::LevelFilter::Trace => LevelFilter::Trace,
            simplelog::LevelFilter::Debug => LevelFilter::Debug,
            simplelog::LevelFilter::Info => LevelFilter::Info,
            simplelog::LevelFilter::Warn => LevelFilter::Warn,
            simplelog::LevelFilter::Error => LevelFilter::Error,
            simplelog::LevelFilter::Off => LevelFilter::Off,
        };

        tide::log::with_level(tide_log_level);
    } else {
        let config = simplelog::ConfigBuilder::new().build();

        if let Err(err) = { simplelog::SimpleLogger::init(opt.log_level, config) } {
            eprintln!("can not initialize logger: {}", err);
            ::std::process::exit(1);
        }
    }

    trace!("opt: {:#?}", opt);

    let config = Config::read_path(opt.config_path)?;

    match opt.cmd {
        SubCommand::Add(sub_opt) => run_add(sub_opt, config),
        SubCommand::Cleanup(sub_opt) => run_cleanup(sub_opt, config),
        SubCommand::Completion(sub_opt) => run_completion(sub_opt),
        SubCommand::Done(sub_opt) => run_done(sub_opt, config),
        SubCommand::Due(sub_opt) => run_due(sub_opt, config),
        SubCommand::Edit(sub_opt) => run_edit(sub_opt, config),
        SubCommand::List(sub_opt) => run_list(sub_opt, config),
        SubCommand::Move(sub_opt) => run_move(sub_opt, config),
        SubCommand::Print(sub_opt) => run_print(sub_opt, config),
        SubCommand::Projects(sub_opt) => run_projects(sub_opt, config),
        SubCommand::Web(sub_opt) => run_web(sub_opt, config).await,
    }
}

fn run_add(opt: AddSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

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

fn run_cleanup(opt: CleanupSubCommandOpts, config: Config) -> Result<(), Error> {
    Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?
    .run_cleanup()
}

fn run_completion(opt: CompletionSubCommandOpts) -> Result<(), Error> {
    std::fs::create_dir_all(&opt.directory)?;
    Opt::clap().gen_completions(env!("CARGO_PKG_NAME"), opt.shell, opt.directory);

    Ok(())
}

fn run_done(opt: DoneSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;
    store.entry_done(opt.entry_id, &opt.project_opt.project)?;

    Ok(())
}

fn run_edit(opt: EditSubCommandOpts, config: Config) -> Result<(), Error> {
    if opt.entry_id < 1 {
        bail!("entry id can not be smaller than 1")
    }

    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

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

fn run_list(opt: ListSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

    let entries = store
        .get_active_entries(&opt.project_opt.project)
        .context("can not get entries from store")?;

    if entries.is_empty() {
        println!("no active todos");
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset("                   ");
    table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("ID").add_attribute(Attribute::Bold),
        Cell::new("Age").add_attribute(Attribute::Bold),
        Cell::new("Due").add_attribute(Attribute::Bold),
        Cell::new("Description").add_attribute(Attribute::Bold),
    ]);

    for (index, entry) in entries.into_iter().enumerate() {
        table.add_row(vec![
            format!("{}", index + 1),
            format_duration(entry.age()),
            format_timestamp(entry.metadata.due),
            format!("{}", entry),
        ]);
    }

    println!("{}", table);

    Ok(())
}

fn run_move(opt: MoveSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

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

fn run_print(opt: PrintSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

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

fn run_projects(opt: ProjectsSubCommandOpts, config: Config) -> Result<(), Error> {
    if opt.simple {
        run_projects_simple(opt, config)
    } else {
        run_projects_normal(opt, config)
    }
}

fn run_projects_simple(opt: ProjectsSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

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

fn run_projects_normal(opt: ProjectsSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

    let mut projects_count = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .filter(|entry| entry.active_count != 0 || opt.print_inactive)
        .collect::<Vec<_>>();

    projects_count.sort();

    let mut table = Table::new();
    table.load_preset("                   ");
    table.set_content_arrangement(comfy_table::ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Project").add_attribute(Attribute::Bold),
        Cell::new("Active").add_attribute(Attribute::Bold),
        Cell::new("Done").add_attribute(Attribute::Bold),
        Cell::new("Total").add_attribute(Attribute::Bold),
    ]);

    for entry in &projects_count {
        trace!("entry written to table: {:#?}", entry);

        table.add_row(vec![
            entry.project.to_string(),
            entry.active_count.to_string(),
            entry.done_count.to_string(),
            entry.total_count.to_string(),
        ]);
    }

    if !projects_count.is_empty() {
        table.add_row(vec!["", "------", "----", "-----"]);
    }

    let total = store
        .get_projects_count()
        .context("can not get projects count from store")?
        .into_iter()
        .fold(ProjectCount::default(), |acc, x| acc + x);

    table.add_row(vec![
        "Total".to_string(),
        total.active_count.to_string(),
        total.done_count.to_string(),
        total.total_count.to_string(),
    ]);

    println!("{}", table);

    Ok(())
}

fn run_due(opt: DueSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

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

async fn run_web(opt: WebSubCommandOpts, config: Config) -> Result<(), Error> {
    let store = Store::open(
        &opt.datadir_opt.datadir,
        config.identifier,
        config.vcs_config,
    )?;

    crate::webservice::WebService::open(store)?
        .run(opt.binding)
        .await?;

    Ok(())
}
