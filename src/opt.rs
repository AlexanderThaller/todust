use chrono::NaiveDate;
use lazy_static::lazy_static;
use simplelog::LevelFilter;
use std::{
    net::SocketAddr,
    path::PathBuf,
};
use structopt::{
    clap::{
        AppSettings::*,
        Shell,
    },
    StructOpt,
};

lazy_static! {
    static ref DEFAULT_DATADIR: PathBuf = xdg::BaseDirectories::with_prefix("todust")
        .expect("can not read xdg base directories")
        .get_data_home();
    static ref DEFAULT_DATADIR_STRING: &'static str = DEFAULT_DATADIR
        .to_str()
        .expect("can not convert xdg data home to string");
    static ref DEFAULT_CONFIG_PATH: PathBuf = xdg::BaseDirectories::with_prefix("todust")
        .expect("can not read xdg base directories")
        .place_config_file("config.toml")
        .expect("can not place config file");
    static ref DEFAULT_CONFIG_PATH_STRING: &'static str = DEFAULT_CONFIG_PATH
        .to_str()
        .expect("can not convert xdg config file path to string");
}

/// Very basic todo cli tool that supports multiline todos.
#[derive(StructOpt, Debug)]
#[structopt(
    settings = &[SubcommandRequiredElseHelp],
    global_settings = &[ColoredHelp, VersionlessSubcommands, NextLineHelp, GlobalVersion]
)]
pub(super) struct Opt {
    /// Loglevel to run under
    #[structopt(
        short = "L",
        long = "log_level",
        global = true,
        value_name = "level",
        default_value = "info",
        possible_values = &["trace", "debug", "info", "warn", "error"],
        env = "TODUST_LOG_LEVEL"
    )]
    pub(super) log_level: LevelFilter,

    /// Config file to use
    #[structopt(
        short = "C",
        long = "config_path",
        global = true,
        value_name = "path",
        default_value = &DEFAULT_CONFIG_PATH_STRING,
        env = "TODUST_CONFIG_PATH"
    )]
    pub(super) config_path: PathBuf,

    /// Subcommand to run
    #[structopt(subcommand)]
    pub(super) cmd: SubCommand,
}

#[derive(StructOpt, Debug)]
pub(super) struct DatadirOpt {
    /// Path to the datadir
    #[structopt(
        short = "d",
        long = "datadir",
        value_name = "path",
        default_value = &DEFAULT_DATADIR_STRING,
        env = "TODUST_DATADIR"
    )]
    pub(super) datadir: PathBuf,
}

#[derive(StructOpt, Debug)]
pub(super) struct ProjectOpt {
    /// Which project to save the entry under
    #[structopt(
        short = "p",
        long = "project",
        value_name = "project",
        default_value = "default",
        env = "TODUST_PROJECT"
    )]
    pub(super) project: String,
}

/// Available subcommands in the application
#[derive(StructOpt, Debug)]
pub(super) enum SubCommand {
    /// Add a new todo entry. If no text is given $EDITOR will be launched.
    #[structopt(name = "add")]
    Add(AddSubCommandOpts),

    /// Cleanup index and unreferenced todos
    #[structopt(name = "cleanup")]
    Cleanup(CleanupSubCommandOpts),

    /// Print formatted todos
    #[structopt(name = "print")]
    Print(PrintSubCommandOpts),

    /// List active todos
    #[structopt(name = "list")]
    List(ListSubCommandOpts),

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

    /// Set due date for entry
    #[structopt(name = "due")]
    Due(DueSubCommandOpts),

    /// Generate shell completion for todust
    #[structopt(name = "completion")]
    Completion(CompletionSubCommandOpts),

    /// Launch webservice
    #[structopt(name = "web")]
    Web(WebSubCommandOpts),
}

/// Options for the add subcommand
#[derive(StructOpt, Debug)]
pub(super) struct AddSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Text of the entry
    #[structopt(index = 1, value_name = "text")]
    pub(super) text: Option<String>,
}

/// Options for the cleanup subcommand
#[derive(StructOpt, Debug)]
pub(super) struct CleanupSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,
}

/// Options for done subcommand
#[derive(StructOpt, Debug)]
pub(super) struct DoneSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Id of the task that should be marked as done
    #[structopt(index = 1, value_name = "id")]
    pub(super) entry_id: usize,
}

/// Options for edit subcommand
#[derive(StructOpt, Debug)]
pub(super) struct EditSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Id of the task
    #[structopt(index = 1, value_name = "id")]
    pub(super) entry_id: usize,

    /// Update started time of todo to current time if specified
    #[structopt(short = "u", long = "update_time")]
    pub(super) update_time: bool,
}

/// Options for list subcommand
#[derive(StructOpt, Debug)]
pub(super) struct ListSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,
}

/// Options for merge subcommand
#[derive(StructOpt, Debug)]
pub(super) struct MergeIndexFilesSubCommandOpts {
    /// Path to the first input index file
    #[structopt(index = 1, value_name = "index_file_path")]
    pub(super) input_first: PathBuf,

    /// Path to the second input index file
    #[structopt(index = 2, value_name = "index_file_path")]
    pub(super) input_second: PathBuf,

    /// Path to the output index file
    #[structopt(index = 3, value_name = "index_file_path")]
    pub(super) output: PathBuf,

    /// Force overwriting output file
    #[structopt(short = "f", long = "force")]
    pub(super) force: bool,
}

/// Options for move subcommand
#[derive(StructOpt, Debug)]
pub(super) struct MoveSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Id of the task
    #[structopt(index = 1, value_name = "id")]
    pub(super) entry_id: usize,

    /// Target project name
    #[structopt(index = 2, value_name = "project")]
    pub(super) target_project: String,
}

/// Options for print subcommand
#[derive(StructOpt, Debug)]
pub(super) struct PrintSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Id of the task. If none is given all tasks will be printed
    #[structopt(index = 1, value_name = "id")]
    pub(super) entry_id: Option<usize>,

    /// Dont print done tasks if specified
    #[structopt(short = "n", long = "no_done")]
    pub(super) no_done: bool,
}

/// Options for projects subcommand
#[derive(StructOpt, Debug)]
pub(super) struct ProjectsSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Also print out projects without active todos. If not specified inactive
    /// projects will not be listed.
    #[structopt(long = "print_inactive")]
    pub(super) print_inactive: bool,

    /// Only list the projects without statistics or the table formatting.
    /// Usefully for scripts.
    #[structopt(long = "simple")]
    pub(super) simple: bool,
}

/// Options for import subcommand
#[derive(StructOpt, Debug)]
pub(super) struct ImportSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Path of the file/folder from which to import from
    #[structopt(index = 1, value_name = "path")]
    pub(super) from_path: PathBuf,

    /// Import all projects instead of just the current project
    #[structopt(short = "a", long = "import_all")]
    pub(super) import_all: bool,
}

/// Options for due subcommand
#[derive(StructOpt, Debug)]
pub(super) struct DueSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    #[structopt(flatten)]
    pub(super) project_opt: ProjectOpt,

    /// Id of the task for which the due date should be set
    #[structopt(index = 1, value_name = "id")]
    pub(super) entry_id: usize,

    /// When the task is due. Has to be date in format 2019-12-24
    #[structopt(index = 2, value_name = "due_date")]
    pub(super) due_date: NaiveDate,
}

/// Options for completion subcommand
#[derive(StructOpt, Debug)]
pub(super) struct CompletionSubCommandOpts {
    /// Which shell to generate for
    #[structopt(
        short = "s",
        long = "shell",
        value_name = "shell",
        possible_values = &["bash", "fish", "zsh", "elvish"],
    )]
    pub(super) shell: Shell,

    /// Folder to where to save the generated file to
    #[structopt(short = "d", long = "directory", value_name = "path")]
    pub(super) directory: PathBuf,
}

/// Options for the web subcommand
#[derive(StructOpt, Debug)]
pub(super) struct WebSubCommandOpts {
    #[structopt(flatten)]
    pub(super) datadir_opt: DatadirOpt,

    /// Where to bind the webservice to
    #[structopt(
        short = "b",
        long = "binding",
        value_name = "address:port",
        default_value = "127.0.0.1:9216"
    )]
    pub(super) binding: SocketAddr,
}
