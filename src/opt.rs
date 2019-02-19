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
pub struct Opt {
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
    pub log_level: LevelFilter,

    /// Path to the datadir
    #[structopt(
        short = "D",
        long = "datadir",
        raw(global = "true"),
        value_name = "path",
        raw(default_value = "&DEFAULT_DATADIR_STRING"),
        env = "TODUST_DATADIR"
    )]
    pub datadir: PathBuf,

    /// Which project to save the entry under
    #[structopt(
        short = "P",
        long = "project",
        raw(global = "true"),
        value_name = "project",
        default_value = "default",
        env = "TODUST_PROJECT"
    )]
    pub project: String,

    /// Subcommand to run
    #[structopt(subcommand)]
    pub cmd: SubCommand,
}

/// Available subcommands in the application
#[derive(StructOpt, Debug)]
pub enum SubCommand {
    /// Start editor and add note
    #[structopt(name = "add")]
    Add,

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

    /// Migrate old entries to new format
    #[structopt(name = "migrate")]
    Migrate(MigrateSubCommandOpts),

    /// Move entry from current project to target project
    #[structopt(name = "move")]
    Move(MoveSubCommandOpts),

    // FIXME: Disable project flag in this subcommand as it doesnt make sense here.
    /// Print all projects saved in todust
    #[structopt(name = "move")]
    Projects(ProjectsSubCommandOpts),
}

/// Options for print subcommand
#[derive(StructOpt, Debug)]
pub struct PrintSubCommandOpts {
    /// Id of the task. If none is given all tasks will be printed
    #[structopt(index = 1, value_name = "id")]
    pub entry_id: Option<usize>,

    /// Dont print done tasks if specified
    #[structopt(short = "n", long = "no_done")]
    pub no_done: bool,
}

/// Options for done subcommand
#[derive(StructOpt, Debug)]
pub struct DoneSubCommandOpts {
    /// Id of the task that should be marked as done
    #[structopt(index = 1, value_name = "id")]
    pub entry_id: usize,
}

/// Options for edit subcommand
#[derive(StructOpt, Debug)]
pub struct EditSubCommandOpts {
    /// Id of the task
    #[structopt(index = 1, value_name = "id")]
    pub entry_id: usize,

    /// Update started time of todo to current time if specified
    #[structopt(short = "u", long = "update_time")]
    pub update_time: bool,
}

/// Options for migrate subcommand
#[derive(StructOpt, Debug)]
pub struct MigrateSubCommandOpts {
    /// Path of the file/folder from which to migrate from
    #[structopt(index = 1, value_name = "path")]
    pub from_path: PathBuf,
}

/// Options for move subcommand
#[derive(StructOpt, Debug)]
pub struct MoveSubCommandOpts {
    /// Id of the task
    #[structopt(index = 1, value_name = "id")]
    pub entry_id: usize,

    /// Target project name
    #[structopt(index = 2, value_name = "project_name")]
    pub target_project: String,
}

/// Options for projects subcommand
#[derive(StructOpt, Debug)]
pub struct ProjectsSubCommandOpts {
    /// Also print out projects without active todos. If not specified inactive
    /// projects will not be listed
    #[structopt(short = "i", long = "print_inactive")]
    pub print_inactive: bool,
}
