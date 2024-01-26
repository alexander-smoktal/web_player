use std::{
    fs::{read_dir, DirEntry},
    path::Path,
};

use anyhow::{Context, Result};

use serde_json::{json, Value};

pub const VIDEO_URL_PREFIX: &str = "/video/";

pub fn browse_dir(path: &Path, videos_dir: &Path, opened_path: Option<&String>) -> Result<Value> {
    let mut result = make_dir(path, videos_dir, opened_path)?;

    let (mut dirs, mut files): (Vec<_>, Vec<_>) = read_dir(path)?.partition(|entry| {
        entry.is_ok() && entry.as_ref().map(|entry| entry.path().is_dir()).unwrap()
    });

    // We want a nice sorted lists
    sort_direntries(&mut dirs);
    sort_direntries(&mut files);

    // Process dirs first
    for dir in dirs {
        let path = dir?.path();

        result["entries"]
            .as_array_mut()
            .context("Internal error. Not an array")?
            .push(browse_dir(&path, videos_dir, opened_path)?);
    }

    // And files later
    for file in files {
        let path = file?.path();

        result["entries"]
            .as_array_mut()
            .context("Internal error. Not an array")?
            .push(make_file(&path, videos_dir, opened_path)?);
    }

    Ok(result)
}

fn sort_direntries(entries: &mut Vec<std::io::Result<DirEntry>>) {
    entries.sort_by(|left, right| {
        left.as_ref()
            .unwrap()
            .file_name()
            .cmp(&right.as_ref().unwrap().file_name())
    });
}

fn make_dir(path: &Path, videos_dir: &Path, opened_path: Option<&String>) -> Result<Value> {
    // Dir verbose name
    let filename_str = path
        .file_name()
        .context("No dir filename")?
        .to_str()
        .context("Dir filename is not a string")?;

    // Path to the dir
    // Leaves only relative to `videos_dir` path
    let path_str = path
        .to_str()
        .context("Dir path is not a string")?
        .strip_prefix(videos_dir.to_str().unwrap())
        .unwrap()
        .to_owned();

    // Check if current dir is opened to open file menu entry on a web page
    let opened = opened_path.is_some()
        && opened_path
            .unwrap()
            .strip_prefix(VIDEO_URL_PREFIX)
            .context("Internal erro can't strip video URL prefix")?
            .starts_with(&path_str);

    Ok(json!({
        "is_dir": true,
        "type": "dir",
        "name": filename_str,
        "is_opened": opened,
        "entries": []
    }))
}

fn make_file(path: &Path, videos_dir: &Path, opened_path: Option<&String>) -> Result<Value> {
    // Verbose file name
    let filename_str = path
        .file_name()
        .context("No dir filename")?
        .to_str()
        .context("Video filename is not a string")?;

    // Path to the video file
    // Leaves only relative to `videos_dir` path
    let path_str = path
        .to_str()
        .context("File path is not a string")?
        .strip_prefix(videos_dir.to_str().unwrap())
        .unwrap()
        .to_owned();

    let full_video_url = VIDEO_URL_PREFIX.to_owned() + &path_str;

    let opened = opened_path.is_some() && opened_path.unwrap() == &full_video_url;

    Ok(json!({
        "is_dir": false,
        "type": "file",
        "name": filename_str,
        "path": full_video_url,
        "is_opened": opened,
    }))
}
