use chrono::Duration;
use failure::{
    Error,
    ResultExt,
};
use std::fs::File;
use tempdir::TempDir;

pub fn confirm(message: &str, default: bool) -> Result<bool, Error> {
    let default_text = if default { "Y/n" } else { "N/y" };

    println!("{}\n({}): ", message, default_text);
    let input: String = read!("{}\n");

    match input.trim().to_uppercase().as_str() {
        "Y" | "YES" => Ok(true),
        "N" | "NO" => Ok(false),
        _ => bail!("do not know what to do with {}", input),
    }
}

pub fn string_from_editor(prepoluate: Option<&str>) -> Result<String, Error> {
    use std::env;
    use std::io::{
        Read,
        Write,
    };
    use std::process::Command;

    let tmpdir = TempDir::new("todust_tmp").unwrap();
    let tmppath = tmpdir.path().join("todo.asciidoc");
    let editor = {
        match env::var("VISUAL") {
            Ok(editor) => editor,
            Err(_) => match env::var("EDITOR") {
                Ok(editor) => editor,
                Err(_) => {
                    bail!("not editor set. either set $VISUAL OR $EDITOR environment variable")
                }
            },
        }
    };

    if let Some(content) = prepoluate {
        let mut file = File::create(tmppath.display().to_string())
            .context("can not open tmp editor file to prepoluate with string")?;

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

pub fn format_duration(duration: Duration) -> String {
    if duration < Duration::minutes(1) {
        return format!("{}s", duration.num_seconds());
    }

    if duration < Duration::hours(1) {
        return format!("{}m", duration.num_minutes());
    }

    if duration < Duration::hours(24) {
        return format!("{}h", duration.num_hours());
    }

    format!("{}d", duration.num_days())
}