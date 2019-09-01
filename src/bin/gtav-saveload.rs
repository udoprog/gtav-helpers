#![windows_subsystem = "windows"]

use clap::{App, Arg};
use failure::Error;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn list_save_files(path: &Path) -> Result<Vec<PathBuf>, Error> {
    return find_matching(path, |p| p.is_file(), |n| n.starts_with("SGTA"));
}

/// List files that contains the given name.
fn list_name_contains(path: &Path, name: &str) -> Result<Vec<PathBuf>, Error> {
    return find_matching(path, |p| p.is_dir(), |n| n.contains(name));
}

/// Find files matching the given predicate.
fn find_matching<P, F>(path: &Path, p: P, m: F) -> Result<Vec<PathBuf>, Error>
where
    P: Copy + Fn(&Path) -> bool,
    F: Copy + Fn(&str) -> bool,
{
    let mut out = Vec::new();

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let path = entry.path();

        if p(&path)
            && path
                .file_name()
                .and_then(|n| n.to_str().map(m))
                .unwrap_or(false)
        {
            out.push(path)
        }
    }

    Ok(out)
}

/// Ensure that the Slots directory exists and return it.
fn ensure_slot(path: &Path) -> Result<PathBuf, Error> {
    let slots = path.join("Slots");

    if !slots.is_dir() {
        fs::create_dir(&slots)?;
    }

    Ok(slots)
}

fn move_save_files(from: &Path, to: &Path) -> Result<(), Error> {
    for save_file in list_save_files(&to)? {
        println!("delete: {}", save_file.display());
        fs::remove_file(&save_file)?;
    }

    for save_file in list_save_files(&from)? {
        if let Some(file_name) = save_file.file_name() {
            let dest = to.join(file_name);
            println!("{} -> {}", save_file.display(), dest.display());
            fs::copy(save_file, &dest)?;
        }
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let matches = App::new("GTA V SaveLoad Helper")
        .version(env!("CARGO_PKG_VERSION"))
        .author("John-John Tedro")
        .about("Manages GTA V Save Files")
        .arg(
            Arg::with_name("save")
                .long("save")
                .value_name("slot")
                .help("Saves the current save files in the given slot.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("load")
                .long("load")
                .value_name("slot")
                .help("Loads the current save files in the given slot.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("load-save-file")
                .long("load-save-file")
                .value_name("slot")
                .help("Loads the current save file from the Save Files folder.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("save-dated")
                .long("save-dated")
                .help("Removes the current save files, and saves them in a dated folder."),
        )
        .get_matches();

    let base = PathBuf::from(env::var("USERPROFILE")?)
        .join("Documents")
        .join("Rockstar Games")
        .join("GTA V");

    let profiles = base.join("Profiles");

    if !profiles.is_dir() {
        println!("Missing profile directory: {}", base.display());
        return Ok(());
    }

    let mut existing_profiles = Vec::new();

    for entry in fs::read_dir(&profiles)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            existing_profiles.push(path.to_owned());
        }
    }

    for profile in &existing_profiles {
        if let Some(slot) = matches.value_of("save") {
            let slot = ensure_slot(&profile)?.join(slot);

            if !slot.is_dir() {
                fs::create_dir(&slot)?;
            }

            move_save_files(profile, &slot)?;
        }

        if let Some(slot) = matches.value_of("load") {
            let slot = ensure_slot(&profile)?.join(slot);

            if !slot.is_dir() {
                fs::create_dir(&slot)?;
            }

            move_save_files(&slot, profile)?;
        }

        if let Some(name) = matches.value_of("load-save-file") {
            if let Some(from) = list_name_contains(&profile.join("Save Files"), name)?.first() {
                move_save_files(&from, profile)?;
            }
        }

        if matches.is_present("save-dated") {
            let when = chrono::Local::now();
            let when = format!("dated-{}", when.format("%Y-%m-%d_%H%M%S"));
            let slot = ensure_slot(&profile)?.join(when);

            if !slot.is_dir() {
                fs::create_dir(&slot)?;
            }

            move_save_files(profile, &slot)?;

            for save_file in list_save_files(&profile)? {
                println!("delete: {}", save_file.display());
                fs::remove_file(&save_file)?;
            }
        }
    }

    Ok(())
}
