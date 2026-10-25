//! Reading and writing the library's files: summaries, backups' stamps,
//! atomic writes, and the checks that keep ids inside the library folder.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::SystemTime;

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;

use super::{migrate, ProjectSummary};
use crate::project::{Project, PROJECT_EXTENSION};
use crate::{Error, Result};

/// The few fields the project list needs. `IgnoredAny` skips the scene without
/// building it, for a file whose scene does not come last.
#[derive(Deserialize)]
struct SummaryFields {
    name: String,
    modified: String,
    #[serde(default)]
    scene: Option<IgnoredAny>,
    #[serde(default)]
    look: Option<Value>,
}

/// How much of a project file is read for its summary before the whole file
/// is. Everything before the scene takes a few kB; the scene takes megabytes.
const HEAD_BYTES: u64 = 256 * 1024;

/// The scene's key in a compact project file. `Project` writes the scene last,
/// so every field a summary needs comes before it.
const SCENE_KEY: &[u8] = b",\"scene\":";

/// A save writes here first and renames over the real file when complete.
const TEMP_EXTENSION: &str = "tmp";

/// Longest name slug inside a project id, so file names stay readable.
const SLUG_MAX_CHARS: usize = 40;

pub(super) fn read_project(path: &Path) -> Result<Project> {
    let format_error = |detail: String| Error::Format {
        path: path.display().to_string(),
        detail,
    };

    let bytes = fs::read(path).map_err(|e| Error::io(path, e))?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| format_error(e.to_string()))?;
    migrate(value).map_err(|e| format_error(e.to_string()))
}

/// Every project file in `folder` with its file time. A file that cannot be
/// read becomes an entry with `error` set, so one bad file never hides the rest.
pub(super) fn read_summaries(folder: &Path) -> Result<Vec<(ProjectSummary, SystemTime)>> {
    let entries = fs::read_dir(folder).map_err(|e| Error::io(folder, e))?;
    let mut summaries = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| Error::io(folder, e))?.path();
        if path.extension().and_then(|e| e.to_str()) != Some(PROJECT_EXTENSION) {
            continue;
        }

        let file_time = fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let summary = summarize(&path).unwrap_or_else(|e| unreadable(&path, file_time, e));
        summaries.push((summary, file_time));
    }

    Ok(summaries)
}

pub(super) fn summarize(path: &Path) -> Result<ProjectSummary> {
    // A file saved before the scene moved last has its look after the scene.
    let (fields, has_scene) = match head_fields(path)? {
        Some((fields, has_scene)) if fields.look.is_some() => (fields, has_scene),
        _ => {
            let fields = whole_fields(path)?;
            let has_scene = fields.scene.is_some();
            (fields, has_scene)
        }
    };

    Ok(ProjectSummary {
        id: file_stem(path),
        name: fields.name,
        modified: fields.modified,
        has_scene,
        // Summaries skip migration to stay fast, so a look in an older shape
        // is left out rather than failing the whole entry.
        look: fields
            .look
            .and_then(|look| serde_json::from_value(look).ok()),
        error: None,
    })
}

/// The `modified` saved in a project file, as written.
pub(super) fn read_modified(path: &Path) -> Result<String> {
    match head_fields(path)? {
        Some((fields, _)) => Ok(fields.modified),
        None => Ok(whole_fields(path)?.modified),
    }
}

/// The fields before the scene, and whether there is a scene, read from the
/// start of the file only. `None` when that is not enough: the scene does not
/// start within `HEAD_BYTES`, or a longer file has none.
fn head_fields(path: &Path) -> Result<Option<(SummaryFields, bool)>> {
    let file = fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut head = Vec::new();
    file.take(HEAD_BYTES)
        .read_to_end(&mut head)
        .map_err(|e| Error::io(path, e))?;
    let scene_at = head
        .windows(SCENE_KEY.len())
        .position(|window| window == SCENE_KEY);

    let Some(scene_at) = scene_at else {
        let is_whole_file = (head.len() as u64) < HEAD_BYTES;
        if !is_whole_file {
            return Ok(None);
        }
        let fields = parse_fields(path, &head)?;
        let has_scene = fields.scene.is_some();
        return Ok(Some((fields, has_scene)));
    };
    // A key named "scene" deeper in the file leaves an object open, so this
    // does not parse and the whole file is read instead.
    let mut before_scene = head[..scene_at].to_vec();
    before_scene.push(b'}');
    let Ok(fields) = serde_json::from_slice::<SummaryFields>(&before_scene) else {
        return Ok(None);
    };
    let has_scene = !head[scene_at + SCENE_KEY.len()..].starts_with(b"null");

    Ok(Some((fields, has_scene)))
}

fn whole_fields(path: &Path) -> Result<SummaryFields> {
    let bytes = fs::read(path).map_err(|e| Error::io(path, e))?;
    parse_fields(path, &bytes)
}

fn parse_fields(path: &Path, bytes: &[u8]) -> Result<SummaryFields> {
    serde_json::from_slice(bytes).map_err(|e| Error::Format {
        path: path.display().to_string(),
        detail: e.to_string(),
    })
}

pub(super) fn unreadable(path: &Path, file_time: SystemTime, error: Error) -> ProjectSummary {
    ProjectSummary {
        id: file_stem(path),
        name: file_stem(path),
        modified: humantime::format_rfc3339_nanos(file_time).to_string(),
        has_scene: false,
        look: None,
        error: Some(error.to_string()),
    }
}

/// The stamps of every backup in `folder`, in no particular order.
pub(super) fn backup_stamps(folder: &Path) -> Result<Vec<String>> {
    if !folder.exists() {
        return Ok(Vec::new());
    }

    let mut stamps = Vec::new();
    for entry in fs::read_dir(folder).map_err(|e| Error::io(folder, e))? {
        let path = entry.map_err(|e| Error::io(folder, e))?.path();
        if path.extension().and_then(|e| e.to_str()) == Some(PROJECT_EXTENSION) {
            stamps.push(file_stem(&path));
        }
    }
    Ok(stamps)
}

/// The `modified` time saved in a project file, normalised to the format
/// `now_rfc3339` writes. A file too broken to read falls back to now, because
/// it still has to be backed up before it is replaced.
pub(super) fn saved_modified(path: &Path) -> String {
    read_modified(path)
        .ok()
        .and_then(|modified| humantime::parse_rfc3339_weak(&modified).ok())
        .map(|time| humantime::format_rfc3339_nanos(time).to_string())
        .unwrap_or_else(now_rfc3339)
}

/// Writes to a temporary file next to `path`, then renames it over `path`. A
/// crash mid-write leaves the old file whole, because a rename within one
/// folder replaces the file in one step. A temp file left behind by a crash is
/// harmless: lists skip it and the next save overwrites it.
pub(super) fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(TEMP_EXTENSION);
    let mut file = fs::File::create(&temp).map_err(|e| Error::io(&temp, e))?;
    file.write_all(bytes).map_err(|e| Error::io(&temp, e))?;
    file.sync_all().map_err(|e| Error::io(&temp, e))?;
    // Windows cannot rename a file that is still open.
    drop(file);

    fs::rename(&temp, path).map_err(|e| Error::io(path, e))
}

/// UTC with nanoseconds, always the same width: "2026-10-24T15:30:12.123456789Z".
pub(super) fn now_rfc3339() -> String {
    humantime::format_rfc3339_nanos(SystemTime::now()).to_string()
}

/// "2026-10-24T15:30:12.123456789Z" -> "2026-10-24T15-30-12.123456789Z".
/// Windows does not allow ':' in file names.
pub(super) fn stamp_from_rfc3339(time: &str) -> String {
    time.replace(':', "-")
}

/// "2026-10-24T15-30-12.123456789Z" -> "2026-10-24T15:30:12.123456789Z".
pub(super) fn rfc3339_from_stamp(stamp: &str) -> String {
    match stamp.split_once('T') {
        Some((date, time)) => format!("{date}T{}", time.replace('-', ":")),
        None => stamp.to_string(),
    }
}

/// "Westermarkt 20, Amsterdam" -> "westermarkt-20-amsterdam", and `fallback`
/// for a name with no ASCII letters or digits in it.
pub(super) fn slugify(name: &str, fallback: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if slug.len() >= SLUG_MAX_CHARS {
            break;
        }
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }

    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        fallback.to_string()
    } else {
        slug.to_string()
    }
}

/// "<stem>.openarchi", once the stem is known to stay inside its folder.
pub(super) fn file_name(stem: &str) -> Result<String> {
    check_file_stem(stem)?;
    Ok(format!("{stem}.{PROJECT_EXTENSION}"))
}

/// Ids and stamps arrive from the app and from assistants, so make sure one
/// can never point outside the library.
pub(super) fn check_file_stem(stem: &str) -> Result<()> {
    let is_safe = !stem.is_empty()
        && !stem.starts_with('.')
        && !stem.contains(['/', '\\', ':'])
        && !stem.contains("..");
    if is_safe {
        Ok(())
    } else {
        Err(Error::Invalid(format!("not a valid project id: {stem:?}")))
    }
}

pub(super) fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}
