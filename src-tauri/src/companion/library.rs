//! Imported companions on disk (DF-0025, ADR-0015): one folder or `.zip`
//! per pack in `<config folder>/companions`, read at start, after an import
//! or a removal, and when the settings come back to the front if a file
//! changed.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use super::pack::{Definition, MAX_FILE_BYTES, Manifest, PackError};
use super::{Custom, set_custom};
use crate::AppState;

const FOLDER: &str = "companions";
const MANIFEST: &str = "companion.json";
const PREFIX: &str = "custom:";
/// Shipped in the template pack: how to make one.
const README: &str = include_str!("template-readme.md");

/// A pack found on disk.
enum Source {
    Folder(PathBuf),
    Zip(PathBuf),
}

/// A valid pack.
struct Pack {
    name: String,
    sheet: String,
    definition: Definition,
}

fn folder(config_dir: &Path) -> PathBuf {
    config_dir.join(FOLDER)
}

fn read(source: &Source) -> Result<Pack, PackError> {
    match source {
        Source::Folder(dir) => {
            let manifest =
                Manifest::parse(&read_file(&dir.join(MANIFEST), PackError::MissingManifest)?)?;
            let sheet = read_file(
                &dir.join(&manifest.sheet),
                PackError::MissingSheet(manifest.sheet.clone()),
            )?;
            Ok(Pack {
                name: manifest.name(),
                definition: manifest.build(&sheet)?,
                sheet: manifest.sheet,
            })
        }
        Source::Zip(path) => read_zip(&read_file(path, PackError::InvalidZip)?),
    }
}

fn read_file(path: &Path, missing: PackError) -> Result<Vec<u8>, PackError> {
    let meta = fs::metadata(path).map_err(|_| missing.clone())?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(PackError::TooBig);
    }
    fs::read(path).map_err(|_| missing)
}

/// A zipped pack: `companion.json` at its root, or in a single folder (a
/// zipped folder).
fn read_zip(bytes: &[u8]) -> Result<Pack, PackError> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| PackError::InvalidZip)?;
    let manifest_name = archive
        .file_names()
        .filter(|name| {
            *name == MANIFEST
                || name
                    .strip_suffix(MANIFEST)
                    .and_then(|dir| dir.strip_suffix('/'))
                    .is_some_and(|dir| !dir.is_empty() && !dir.contains('/'))
        })
        .min_by_key(|name| name.len())
        .map(str::to_owned)
        .ok_or(PackError::MissingManifest)?;
    let prefix = manifest_name[..manifest_name.len() - MANIFEST.len()].to_owned();
    let manifest = Manifest::parse(
        &zip_entry(&mut archive, &manifest_name)?.ok_or(PackError::MissingManifest)?,
    )?;
    let sheet = zip_entry(&mut archive, &format!("{prefix}{}", manifest.sheet))?
        .ok_or_else(|| PackError::MissingSheet(manifest.sheet.clone()))?;
    Ok(Pack {
        name: manifest.name(),
        definition: manifest.build(&sheet)?,
        sheet: manifest.sheet,
    })
}

fn zip_entry(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<Vec<u8>>, PackError> {
    let Ok(file) = archive.by_name(name) else {
        return Ok(None);
    };
    if file.size() > MAX_FILE_BYTES {
        return Err(PackError::TooBig);
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PackError::InvalidZip)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(PackError::TooBig);
    }
    Ok(Some(bytes))
}

/// The packs in `dir`, by file name: folders and `.zip` files.
fn sources(dir: &Path) -> Vec<(String, Source)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Source)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                None
            } else if path.is_dir() {
                Some((name, Source::Folder(path)))
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
            {
                Some((name, Source::Zip(path)))
            } else {
                None
            }
        })
        .collect();
    found.sort_by_key(|(name, _)| name.to_lowercase());
    found
}

fn scan(dir: &Path) -> Vec<Custom> {
    sources(dir)
        .into_iter()
        .map(|(file, source)| {
            let id = format!("{PREFIX}{file}");
            match read(&source) {
                Ok(pack) => Custom {
                    id,
                    name: pack.name,
                    definition: Some(pack.definition),
                    error: None,
                },
                Err(e) => {
                    log::warn!("companion pack {file} refused: {e:?}");
                    Custom {
                        id,
                        name: file,
                        definition: None,
                        error: Some(e.message()),
                    }
                }
            }
        })
        .collect()
}

/// What the folder holds (names, sizes, dates, one level into each pack's
/// folder): unchanged, nothing is read again.
fn signature(dir: &Path) -> Vec<(String, u64, u64)> {
    fn stamp(path: &Path, name: String, out: &mut Vec<(String, u64, u64)>) {
        if let Ok(meta) = fs::metadata(path) {
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_millis() as u64);
            out.push((name, meta.len(), modified));
        }
    }
    let mut out = Vec::new();
    for (name, source) in sources(dir) {
        match source {
            Source::Zip(path) => stamp(&path, name, &mut out),
            Source::Folder(path) => {
                for entry in fs::read_dir(&path).into_iter().flatten().flatten() {
                    let inner = entry.file_name().to_string_lossy().into_owned();
                    stamp(&entry.path(), format!("{name}/{inner}"), &mut out);
                }
            }
        }
    }
    out.sort();
    out
}

/// The folder's signature when it was last read.
static LAST: Mutex<Option<Vec<(String, u64, u64)>>> = Mutex::new(None);

/// Reads the packs at start, before the modules show anything.
pub fn load(config_dir: &Path) {
    let dir = folder(config_dir);
    *LAST.lock().unwrap() = Some(signature(&dir));
    set_custom(scan(&dir));
}

/// Reads the packs again if a file changed (or always, `force`) and tells
/// every window.
pub fn reload(app: &AppHandle, force: bool) {
    let dir = folder(&app.state::<AppState>().config_dir);
    let now = signature(&dir);
    {
        let mut last = LAST.lock().unwrap();
        if !force && last.as_ref() == Some(&now) {
            return;
        }
        *last = Some(now);
    }
    set_custom(scan(&dir));
    let _ = app.emit("companions-changed", super::custom());
    crate::emit_content(app);
    crate::settings::changed(app);
}

/// A file name from a pack's name: lowercase letters, digits and dashes.
fn slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_end_matches('-').chars().take(40).collect();
    if slug.is_empty() {
        "companion".into()
    } else {
        slug
    }
}

/// `dir/<base><extension>`, numbered if taken.
fn unique(dir: &Path, base: &str, extension: &str) -> PathBuf {
    let mut path = dir.join(format!("{base}{extension}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{base}-{n}{extension}"));
        n += 1;
    }
    path
}

#[tauri::command]
pub fn get_companions() -> Vec<Custom> {
    super::custom()
}

/// The settings came back to the front: a pack may have been dropped in.
#[tauri::command]
pub fn reload_companions(app: AppHandle) {
    reload(&app, false);
}

/// Asks for a `.zip` or a pack's `companion.json`, checks it and copies it
/// into the folder. Its name, or nothing if the user cancelled.
#[tauri::command]
pub async fn import_companion(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Option<String>, String> {
    let Some(picked) = app
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(crate::t!("settings.companions.add"))
        .add_filter(crate::t!("settings.companions.filter"), &["zip", "json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let failed = |e: &dyn std::fmt::Display| {
        log::warn!("cannot import a companion: {e}");
        crate::t!("settings.companions.import_failed")
    };
    let path = picked.into_path().map_err(|e| failed(&e))?;
    let is_manifest = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    let source = match (is_manifest, path.parent()) {
        (true, Some(parent)) => Source::Folder(parent.to_owned()),
        _ => Source::Zip(path.clone()),
    };
    let pack = read(&source).map_err(|e| e.message())?;
    let dir = folder(&app.state::<AppState>().config_dir);
    fs::create_dir_all(&dir).map_err(|e| failed(&e))?;
    let base = slug(&pack.name);
    match &source {
        Source::Folder(from) => {
            let to = unique(&dir, &base, "");
            fs::create_dir(&to).map_err(|e| failed(&e))?;
            for file in [MANIFEST, pack.sheet.as_str()] {
                fs::copy(from.join(file), to.join(file)).map_err(|e| failed(&e))?;
            }
        }
        Source::Zip(from) => {
            fs::copy(from, unique(&dir, &base, ".zip")).map_err(|e| failed(&e))?;
        }
    }
    reload(&app, true);
    Ok(Some(pack.name))
}

/// Removes an imported companion's pack.
#[tauri::command]
pub fn delete_companion(app: AppHandle, id: String) -> Result<(), String> {
    let failed = || crate::t!("settings.companions.delete_failed");
    let name = id
        .strip_prefix(PREFIX)
        .filter(|n| !n.is_empty() && !n.contains(['/', '\\']) && !n.starts_with('.'))
        .ok_or_else(failed)?;
    let path = folder(&app.state::<AppState>().config_dir).join(name);
    let result = if path.is_dir() {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_file(&path)
    };
    result.map_err(|e| {
        log::warn!("cannot delete companion {name}: {e}");
        failed()
    })?;
    reload(&app, true);
    Ok(())
}

/// Opens the packs' folder (created if needed).
#[tauri::command]
pub fn reveal_companions(app: AppHandle) -> Result<(), String> {
    let dir = folder(&app.state::<AppState>().config_dir);
    fs::create_dir_all(&dir)
        .map_err(|e| e.to_string())
        .and_then(|_| tauri_plugin_opener::open_path(&dir, None::<&str>).map_err(|e| e.to_string()))
        .map_err(|e| {
            log::warn!("cannot open the companions folder: {e}");
            crate::t!("settings.companions.open_failed")
        })
}

/// Saves a template pack made by the front from a built-in companion (its
/// sheet and manifest), with the README, where the user chooses. Whether it
/// was saved.
#[tauri::command]
pub async fn export_companion_template(
    app: AppHandle,
    window: WebviewWindow,
    sheet: Vec<u8>,
    manifest: String,
) -> Result<bool, String> {
    let failed = |e: &dyn std::fmt::Display| {
        log::warn!("cannot export the companion template: {e}");
        crate::t!("settings.companions.export_failed")
    };
    // The template must import as is.
    Manifest::parse(manifest.as_bytes())
        .and_then(|m| m.build(&sheet))
        .map_err(|e| failed(&format!("{e:?}")))?;
    let Some(picked) = app
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(crate::t!("settings.companions.export"))
        .set_file_name("bloop.zip")
        .add_filter(crate::t!("settings.companions.filter"), &["zip"])
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let path = picked.into_path().map_err(|e| failed(&e))?;
    let bytes = template_zip(&sheet, &manifest).map_err(|e| failed(&e))?;
    fs::write(&path, bytes).map_err(|e| failed(&e))?;
    Ok(true)
}

fn template_zip(sheet: &[u8], manifest: &str) -> zip::result::ZipResult<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        (MANIFEST, manifest.as_bytes()),
        ("sheet.png", sheet),
        ("README.md", README.as_bytes()),
    ] {
        zip.start_file(name, options)?;
        zip.write_all(bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST_JSON: &str = r#"{
        "name": "Tiny",
        "frameWidth": 12, "frameHeight": 8,
        "animations": { "neutral": { "frames": [0] }, "active": { "frames": [0] } }
    }"#;

    fn png() -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(12, 8, image::Rgba([0, 0, 0, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    fn zipped(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn the_template_zip_imports_as_is() {
        let bytes = template_zip(&png(), MANIFEST_JSON).unwrap();
        assert_eq!(read_zip(&bytes).unwrap().name, "Tiny");
    }

    #[test]
    fn a_zipped_folder_is_read_too() {
        let png = png();
        let bytes = zipped(&[
            ("tiny/companion.json", MANIFEST_JSON.as_bytes()),
            ("tiny/sheet.png", &png),
        ]);
        assert_eq!(read_zip(&bytes).unwrap().sheet, "sheet.png");
        let bytes = zipped(&[("a/b/companion.json", MANIFEST_JSON.as_bytes())]);
        assert_eq!(read_zip(&bytes).err(), Some(PackError::MissingManifest));
        let bytes = zipped(&[("companion.json", MANIFEST_JSON.as_bytes())]);
        assert_eq!(
            read_zip(&bytes).err(),
            Some(PackError::MissingSheet("sheet.png".into()))
        );
        assert_eq!(read_zip(b"nope").err(), Some(PackError::InvalidZip));
    }

    #[test]
    fn the_folder_lists_valid_and_refused_packs() {
        let dir =
            std::env::temp_dir().join(format!("minim-notch-companions-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let packs = folder(&dir);
        fs::create_dir_all(packs.join("tiny")).unwrap();
        fs::write(packs.join("tiny").join(MANIFEST), MANIFEST_JSON).unwrap();
        fs::write(packs.join("tiny").join("sheet.png"), png()).unwrap();
        fs::write(packs.join("broken.zip"), b"nope").unwrap();
        fs::write(packs.join("notes.txt"), b"ignored").unwrap();
        let before = signature(&packs);
        let found = scan(&packs);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].id, "custom:broken.zip");
        assert!(found[0].definition.is_none() && found[0].error.is_some());
        assert_eq!(
            (found[1].id.as_str(), found[1].name.as_str()),
            ("custom:tiny", "Tiny")
        );
        assert!(found[1].definition.is_some());
        assert_eq!(signature(&packs), before);
        fs::write(packs.join("tiny").join("sheet.png"), b"changed").unwrap();
        assert_ne!(signature(&packs), before);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn imported_packs_get_a_free_file_name() {
        assert_eq!(slug("  Mon Chat — v2! "), "mon-chat-v2");
        assert_eq!(slug("✨"), "companion");
        let dir = std::env::temp_dir().join(format!("minim-notch-unique-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("cat.zip"), b"").unwrap();
        assert_eq!(unique(&dir, "cat", ".zip"), dir.join("cat-2.zip"));
        assert_eq!(unique(&dir, "dog", ".zip"), dir.join("dog.zip"));
        let _ = fs::remove_dir_all(&dir);
    }
}
