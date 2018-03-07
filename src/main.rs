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

extern crate tempdir;

mod todo;

use clap::ArgMatches;
use failure::{
    Context,
    Error,
    ResultExt,
};
use std::fs::OpenOptions;
use std::path::PathBuf;
use todo::{
    Entries,
    Entry,
};

fn main() {
    if let Err(e) = run() {
        for cause in e.causes() {
            error!("{}", cause);
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
        Some("add") => run_add(
            matches
                .subcommand_matches("add")
                .expect("can not get subcommand matches for add"),
        ),
        Some("print") => run_print(
            matches
                .subcommand_matches("print")
                .expect("can not get subcommand matches for print"),
        ),
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

    let mut wtr = csv::WriterBuilder::new()
        .has_headers(new_file)
        .from_writer(file);

    wtr.serialize(entry)
        .context("can not serialize entry to csv")?;

    wtr.flush().context("can not flush csv writer")?;

    Ok(())
}

fn run_print(matches: &ArgMatches) -> Result<(), Error> {
    let datafile_path: PathBuf = matches
        .value_of("datafile_path")
        .ok_or_else(|| Context::new("can not get datafile_path from args"))?
        .into();

    let mut rdr = csv::ReaderBuilder::new()
        .from_path(datafile_path)
        .context("can not create entry reader")?;

    let entries: Entries = rdr.deserialize()
        .filter(|result| result.is_ok())
        .map(|result| result.unwrap())
        .collect();

    println!("{}", entries);

    Ok(())
}

pub fn string_from_editor(prepoluate: Option<&str>) -> Result<String, Error> {
    use std::env;
    use std::fs::File;
    use std::io::{
        Read,
        Write,
    };
    use std::process::Command;
    use tempdir::TempDir;

    let tmpdir = TempDir::new("todust_tmp").unwrap();
    let tmppath = tmpdir.path().join("note.asciidoc");
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

        file.write_all(content.as_bytes())
            .context("can not prepoluate editor tmp file")?;
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

    file.read_to_string(&mut string)
        .context("can not read tmpfile to string")?;

    Ok(string)
}
