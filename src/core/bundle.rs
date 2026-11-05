use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;

use flate2::read::GzDecoder;
use steel_core::ipc::updater::app_bundle_path;
use steel_core::settings::{Loadable, Settings, SETTINGS_FILE_NAME};

use crate::core::http::token_storage::get_token_storage_path;

const BUNDLE_NAME: &str = "steel.app";
const STAGING_DIR_NAME: &str = ".steel-update";

static PENDING_MIGRATION: Mutex<Option<(PathBuf, PathBuf)>> = Mutex::new(None);

pub fn install_target() -> io::Result<PathBuf> {
    let Some(bundle) = app_bundle_path() else {
        return migration_target();
    };
    staging_dir(bundle).map_err(|e| {
        io::Error::other(format!(
            "can't write next to {bundle:?} ({e}) -- move steel.app to the Applications folder to enable updates"
        ))
    })?;
    Ok(bundle.to_path_buf())
}

pub fn install(data: Vec<u8>) -> io::Result<()> {
    let target = install_target()?;
    let update = unpack(data, &target)?;
    if app_bundle_path().is_none() {
        log::info!("updater: {update:?} will be moved to {target:?} on restart");
        *PENDING_MIGRATION.lock().unwrap() = Some((update, target));
        return Ok(());
    }
    log::info!("updater: replacing {target:?} with {update:?}");
    replace(&update, &target)
}

pub fn relaunch_target(executable_path: Option<&Path>) -> io::Result<Option<PathBuf>> {
    if let Some(bundle) = app_bundle_path() {
        return Ok(Some(bundle.to_path_buf()));
    }
    let Some((update, target)) = PENDING_MIGRATION.lock().unwrap().take() else {
        return Ok(None);
    };

    let legacy_dir = std::env::current_dir()?;
    log::info!("restart: moving to {target:?}, data is taken from {legacy_dir:?}");
    replace(&update, &target)?;
    migrate_data(&legacy_dir, &crate::core::os::app_support_dir()?)?;
    if let Some(exe) = executable_path {
        replace_with_launcher(exe, &target)?;
    }
    for name in [PathBuf::from(SETTINGS_FILE_NAME), get_token_storage_path()] {
        let path = legacy_dir.join(name);
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("failed to remove {path:?} after moving it: {e:?}");
            }
        }
    }
    Ok(Some(target))
}

pub fn cleanup_after_update() {
    if let Some(bundle) = app_bundle_path() {
        let staging = bundle.with_file_name(STAGING_DIR_NAME);
        if staging.exists() {
            match std::fs::remove_dir_all(&staging) {
                Ok(()) => log::debug!("removed {staging:?} left after the update"),
                Err(e) => log::warn!("failed to remove {staging:?} left after the update: {e:?}"),
            }
        }
    }
}

fn staging_dir(target: &Path) -> io::Result<PathBuf> {
    let staging = target.with_file_name(STAGING_DIR_NAME);
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir(&staging)?;
    Ok(staging)
}

fn unpack(data: Vec<u8>, target: &Path) -> io::Result<PathBuf> {
    let staging = staging_dir(target)?;
    tar::Archive::new(GzDecoder::new(io::Cursor::new(data))).unpack(&staging)?;

    let update = staging.join(BUNDLE_NAME);
    let output = std::process::Command::new(update.join("Contents/MacOS/steel"))
        .arg("--version")
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "the downloaded app failed to start ({})",
            output.status
        )));
    }
    log::info!(
        "updater: unpacked {update:?}, version {}",
        String::from_utf8_lossy(&output.stdout).trim()
    );
    Ok(update)
}

fn replace(update: &Path, target: &Path) -> io::Result<()> {
    if !target.exists() {
        return std::fs::rename(update, target);
    }
    let previous = update.with_file_name("previous.app");
    std::fs::rename(target, &previous)?;
    if let Err(e) = std::fs::rename(update, target) {
        std::fs::rename(&previous, target)?;
        return Err(e);
    }
    Ok(())
}

fn migration_target() -> io::Result<PathBuf> {
    let system = Path::new("/Applications").join(BUNDLE_NAME);
    if staging_dir(&system).is_ok() {
        return Ok(system);
    }
    let user = crate::core::os::home_dir()?.join("Applications");
    std::fs::create_dir_all(&user)?;
    Ok(user.join(BUNDLE_NAME))
}

fn migrate_data(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    let mut settings = Settings::from_file(&from.join(SETTINGS_FILE_NAME).to_string_lossy())
        .map_err(io::Error::other)?;

    let chat_logs = PathBuf::from(&settings.logging.chat.directory);
    if chat_logs.is_relative() {
        let source = from.join(&chat_logs);
        let movable = chat_logs
            .components()
            .all(|c| matches!(c, Component::Normal(_)));
        let moved =
            movable && (!source.exists() || std::fs::rename(&source, to.join(&chat_logs)).is_ok());
        if !moved {
            settings.logging.chat.directory = source.to_string_lossy().into_owned();
        }
    }

    let token = get_token_storage_path();
    if from.join(&token).exists() {
        backup(&to.join(&token))?;
        std::fs::copy(from.join(&token), to.join(&token))?;
    }

    let destination = to.join(SETTINGS_FILE_NAME);
    backup(&destination)?;
    settings
        .to_file(&destination.to_string_lossy())
        .map_err(io::Error::other)
}

fn backup(path: &Path) -> io::Result<()> {
    if path.exists() {
        let mut backup = path.as_os_str().to_owned();
        backup.push(".bak");
        std::fs::rename(path, backup)?;
    }
    Ok(())
}

fn replace_with_launcher(exe: &Path, bundle: &Path) -> io::Result<()> {
    let launcher = exe.with_extension("launcher");
    std::fs::write(
        &launcher,
        format!("#!/bin/sh\nexec open -a '{}'\n", bundle.display()),
    )?;
    std::fs::set_permissions(&launcher, std::fs::Permissions::from_mode(0o755))?;
    std::fs::rename(&launcher, exe)
}
