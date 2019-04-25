use chrono::NaiveDate;
use lazy_static::lazy_static;
use simplelog::LevelFilter;
use std::path::PathBuf;
use structopt::{
    clap::AppSettings::*,
    StructOpt,
};

lazy_static! {
    static ref DEFAULT_DATADIR: PathBuf = xdg::BaseDirectories::with_prefix("todust")
        .expect("can not read xdg base directories")
        .get_data_home();
    static ref DEFAULT_DATADIR_STRING: &'static str = DEFAULT_DATADIR
        .to_str()
        .expect("can not convert xdg data home to string");
}

/// Very basic todo cli tool that supports multiline todos.
#[derive(StructOpt, Debug)]
#[structopt(
    raw(settings = "&[SubcommandRequiredElseHelp]"),
    raw(global_settings = "&[ColoredHelp, VersionlessSubcommands, NextLineHelp, GlobalVersion]")
)]
pub(crate) struct Opt {
    /// Loglevel to run under
    #[structopt(
        short = "L",
        long = "log_level",
        raw(global = "true"),
        value_name = "level",
        default_value = "info",
        raw(possible_values = r#"&["trace", "debug", "info", "warn", "error"]"#),
        env = "TODUST_LOG_LEVEL"
    )]
    pub(crate) log_level: LevelFilter,

    /// Path to the datadir
    #[structopt(
        short = "D",
        long = "datadir",
        raw(global = "true"),
        value_name = "path",
        raw(default_value = "&DEFAULT_DATADIR_STRING"),
        env = "TODUST_DATADIR"
    )]
    pub(crate) datadir: PathBuf,

    /// Which project to save the entry under
    #[structopt(
        short = "P",
        long = "project",
        raw(global = "true"),
        value_name = "project",
        default_value = "default",
        env = "TODUST_PROJECT"
    )]
    pub(crate) project: String,

    /// Subcommand to run
    #[structopt(subcommand)]
    pub(crate) cmd: SubCommand,
}

/// Available subcommands in the application
#[derive(StructOpt, Debug)]
pub(crate) enum SubCommand {
    /// Add a new todo entry. If no text is given $EDITOR will be launched.
    #[structopt(name = "add")]
    Add(AddSubCommandOpts),

    /// Print formatted todos
    #[structopt(name = "print")]
    Print(PrintSubCommandOpts),

    /// List active todos
    #[structopt(name = "list")]
    List,

    /// Mark entry as done
    #[structopt(name = "done")]
    Done(DoneSubCommandOpts),

    /// Open text of entry in editor to edit it
    #[structopt(name = "edit")]
    Edit(EditSubCommandOpts),

    /// Move entry from current project to target project
    #[structopt(name = "move")]
    Move(MoveSubCommandOpts),

    // FIXME: Disable project flag in this subcommand as it doesnt make sense here.
    /// Print all projects saved in todust
    #[structopt(name = "projects")]
    Projects(ProjectsSubCommandOpts),

    /// Cleanup index and unreferenced todos
    #[structopt(name = "cleanup")]
    Cleanup,

    /// Import entries from a different store
    #[structopt(name = "import")]
    Import(ImportSubCommandOpts),

    /// Set due date for entry
    #[structopt(name = "due")]
    Due(DueSubCommandOpts),
}

/// Options for the add subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct AddSubCommandOpts {
    /// Text of the entry
    #[structopt(index = 1, value_name = "text")]
    pub(crate) text: Option<String>,
}

/// Options for print subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct PrintSubCommandOpts {
    /// Id of the task. If none is given all tasks will be printed
    #[structopt(index = 1, value_name = "id")]
    pub(crate) entry_id: Option<usize>,

    /// Dont print done tasks if specified
    #[structopt(short = "n", long = "no_done")]
    pub(crate) no_done: bool,
}

/// Options for done subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct DoneSubCommandOpts {
    /// Id of the task that should be marked as done
    #[structopt(index = 1, value_name = "id")]
    pub(crate) entry_id: usize,
}

/// Options for edit subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct EditSubCommandOpts {
    /// Id of the task
    #[structopt(index = 1, value_name = "id")]
    pub(crate) entry_id: usize,

    /// Update started time of todo to current time if specified
    #[structopt(short = "u", long = "update_time")]
    pub(crate) update_time: bool,
}

/// Options for move subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct MoveSubCommandOpts {
    /// Id of the task
    #[structopt(index = 1, value_name = "id")]
    pub(crate) entry_id: usize,

    /// Target project name
    #[structopt(index = 2, value_name = "project")]
    pub(crate) target_project: String,
}

/// Options for projects subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct ProjectsSubCommandOpts {
    /// Also print out projects without active todos. If not specified inactive
    /// projects will not be listed
    #[structopt(short = "i", long = "print_inactive")]
    pub(crate) print_inactive: bool,
}

/// Options for import subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct ImportSubCommandOpts {
    /// Path of the file/folder from which to import from
    #[structopt(index = 1, value_name = "path")]
    pub(crate) from_path: PathBuf,

    /// Import all projects instead of just the current project
    #[structopt(short = "a", long = "import_all")]
    pub(crate) import_all: bool,
}

/// Options for due subcommand
#[derive(StructOpt, Debug)]
pub(crate) struct DueSubCommandOpts {
    /// Id of the task for which the due date should be set
    #[structopt(index = 1, value_name = "id")]
    pub(crate) entry_id: usize,

    /// When the task is due. Has to be date in format 2019-12-24
    #[structopt(index = 2, value_name = "due_date")]
    pub(crate) due_date: NaiveDate,
}
