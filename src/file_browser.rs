use std::{fs::read_dir, path::Path};

use anyhow::{Context, Result};

use serde_json::{json, Value};

pub fn browse_dir(path: &Path, opened_path: Option<&String>) -> Result<Value> {
    let path_str = path.to_str().context("File path is not a string")?;
    let filename_str = path
        .file_name()
        .context("No dir filename")?
        .to_str()
        .context("Dir filename is not a string")?;

    let mut result = make_dir(filename_str, path_str, opened_path);

    for entry in read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            result["entries"]
                .as_array_mut()
                .context("Internal error. Not an array")?
                .push(browse_dir(&path, opened_path)?);
        } else {
            let filename_str = path
                .file_name()
                .context("No dir filename")?
                .to_str()
                .context("Dir filename is not a string")?;
            let path_str = path.to_str().context("File path is not a string")?;

            result["entries"]
                .as_array_mut()
                .context("Internal error. Not an array")?
                .push(make_file(filename_str, path_str));
        }
    }

    Ok(result)
}

fn make_dir(name: &str, path: &str, opened_path: Option<&String>) -> Value {
    let opened = opened_path.is_some() && opened_path.unwrap().starts_with(path);

    json!({
        "is_dir": true,
        "type": "dir",
        "name": name,
        "is_opened": opened,
        "entries": []
    })
}

fn make_file(name: &str, path: &str) -> Value {
    json!({
        "is_dir": false,
        "type": "file",
        "name": name,
        "path": path
    })
}
