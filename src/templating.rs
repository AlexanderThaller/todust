use crate::helper;
use chrono::{
    DateTime,
    Utc,
};
use serde_json::value::{
    to_value,
    Value,
};
use std::collections::HashMap;
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
