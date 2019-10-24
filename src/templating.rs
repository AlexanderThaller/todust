use crate::helper;
use chrono::{
    DateTime,
    Utc,
};
use serde_json::value::{
    to_value,
    Value,
};
use std::{
    collections::HashMap,
    io::Write,
};
use tempfile::tempdir;
use tera::{
    try_get_value,
    Result as TeraResult,
};

#[cfg_attr(feature = "cargo-clippy", allow(clippy::needless_pass_by_value))]
pub(super) fn single_line(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("single_line", "value", String, value);

    let s = s.replace("\n", " ");

    Ok(to_value(&s).unwrap())
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::needless_pass_by_value))]
pub(super) fn lines(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let mut out = String::new();

    let s = try_get_value!("lines", "value", String, value);
    let lines = s.lines();
    let mut is_codeblock = false;
    for line in lines {
        if line == "----" {
            is_codeblock = !is_codeblock;
        }

        out.push_str(line);
        out.push('\n');

        if !is_codeblock {
            out.push('\n');
        }
    }

    Ok(to_value(&out).unwrap())
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::needless_pass_by_value))]
pub(super) fn format_duration_since(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let started = try_get_value!("format_duration_since", "value", DateTime<Utc>, value);
    let duration = Utc::now().signed_duration_since(started);

    Ok(to_value(&helper::format_duration(duration)).unwrap())
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::needless_pass_by_value))]
pub(super) fn asciidoc_to_html(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let input = try_get_value!("asciidoc_to_html", "value", String, value);

    let tmpdir = tempdir().expect("can not create tempdir");
    let tmppath = tmpdir.path().join("output.asciidoc");

    let mut file =
        std::fs::File::create(&tmppath).expect("can not create a new file for asciiformatting");

    file.write_all(input.as_bytes())
        .expect("can not write to asciiformatting file");

    let output = std::process::Command::new("asciidoctor")
        .arg("--safe-mode")
        .arg("safe")
        .arg("--no-header-footer")
        .arg("--out-file")
        .arg("-")
        .arg(tmppath)
        .output()
        .expect("problems while running asciidoctor");

    let out = String::from_utf8_lossy(&output.stdout).into_owned();

    Ok(to_value(&out).unwrap())
}
