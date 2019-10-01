#![windows_subsystem = "windows"]

use clap::{App, Arg};
use failure::Error;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn list_save_files(path: &Path) -> Result<Vec<(String, PathBuf)>, Error> {
    return find_matching(path, |p| p.is_file(), |n| n.starts_with("SGTA"));
}

/// List files that contains the given name.
fn list_name_contains(path: &Path, name: &str) -> Result<Vec<(String, PathBuf)>, Error> {
    return find_matching(path, |p| p.is_dir(), |n| n.contains(name));
}

/// Find files matching the given predicate.
fn find_matching<P, F>(path: &Path, p: P, m: F) -> Result<Vec<(String, PathBuf)>, Error>
where
    P: Copy + Fn(&Path) -> bool,
    F: Copy + Fn(&str) -> bool,
{
    let mut out = Vec::new();

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let path = entry.path();

        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        if p(&path) && m(&name) {
            out.push((name, path))
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

/// Copy save files from `from`, to `to`, deleting any existing save files in `to` in the process.
fn copy_save_files(from: &Path, to: &Path) -> Result<(), Error> {
    delete_save_files(&to)?;

    for (_, save_file) in list_save_files(&from)? {
        if let Some(file_name) = save_file.file_name() {
            let dest = to.join(file_name);
            println!("{} -> {}", save_file.display(), dest.display());
            fs::copy(save_file, &dest)?;
        }
    }

    Ok(())
}

/// Find the nth newest slot.
fn find_newest_slot(profile: &Path, nth: usize) -> Result<Option<PathBuf>, Error> {
    let slots = ensure_slot(&profile)?;
    let slots = find_matching(&slots, |p| p.is_dir(), |_| true)?;

    let mut slots_and_meta = slots
        .into_iter()
        .map(|s| {
            let meta = fs::metadata(&s.1)?;
            Ok((s.1, meta.modified()?))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    slots_and_meta.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(slots_and_meta.get(nth).map(|n| n.0.clone()))
}

/// Delete save files in the given path.
fn delete_save_files(path: &Path) -> Result<(), Error> {
    for (_, save_file) in list_save_files(path)? {
        println!("delete: {}", save_file.display());
        fs::remove_file(&save_file)?;
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
        .arg(
            Arg::with_name("clear-profile")
                .long("clear-profile")
                .help("Removes the current save files."),
        )
        .arg(
            Arg::with_name("load-nth-newest-slot")
                .long("load-nth-newest-slot")
                .value_name("nth")
                .help("Load the nth newest slot.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("delete-nth-newest-slot")
                .long("delete-nth-newest-slot")
                .value_name("nth")
                .help("Delete the nth newest slot.")
                .takes_value(true),
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

            copy_save_files(profile, &slot)?;
        }

        if let Some(slot) = matches.value_of("load") {
            let slot = ensure_slot(&profile)?.join(slot);

            if !slot.is_dir() {
                fs::create_dir(&slot)?;
            }

            copy_save_files(&slot, profile)?;
        }

        if let Some(name) = matches.value_of("load-save-file") {
            let mut matches = list_name_contains(&profile.join("Save Files"), name)?;
            matches.sort_by(|a, b| b.0.cmp(&a.0));

            if let Some((_, from)) = matches.first() {
                copy_save_files(&from, profile)?;
            }
        }

        if matches.is_present("save-dated") {
            let when = chrono::Local::now();
            let when = format!("dated-{}", when.format("%Y-%m-%d_%H%M%S"));
            let slot = ensure_slot(&profile)?.join(when);

            if !slot.is_dir() {
                fs::create_dir(&slot)?;
            }

            copy_save_files(profile, &slot)?;
        }

        if matches.is_present("clear-profile") {
            delete_save_files(&profile)?;
        }

        if let Some(nth) = matches.value_of("load-nth-newest-slot") {
            let nth = str::parse::<usize>(nth)?;

            if let Some(path) = find_newest_slot(&profile, nth)? {
                copy_save_files(&path, &profile)?;
            }
        }

        if let Some(nth) = matches.value_of("delete-nth-newest-slot") {
            let nth = str::parse::<usize>(nth)?;

            if let Some(path) = find_newest_slot(&profile, nth)? {
                delete_save_files(&path)?;

                if let Err(e) = fs::remove_dir(&path) {
                    println!("Failed to remove directory: {}", e);
                }
            }
        }
    }

    Ok(())
}
