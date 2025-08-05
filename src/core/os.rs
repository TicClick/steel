use std::path::PathBuf;

pub fn open_in_file_explorer(target: &str) -> std::io::Result<()> {
    let target = match target.starts_with(".") {
        false => std::path::Path::new(target).to_path_buf(),
        true => {
            let exe_path = std::env::current_exe()?;
            let parent = exe_path.parent().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Executable has no parent directory",
                )
            })?;
            parent.join(target)
        }
    };

    log::debug!("requested to open {target:?}");

    if !target.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path does not exist: {target:?}"),
        ));
    }

    if let Some(p) = target.to_str() {
        let path = p.to_owned();
        let (executable, args) = if cfg!(target_os = "windows") {
            // let file_arg = format!("/select,{}", log_path);
            ("explorer.exe", vec![path])
        } else if cfg!(target_os = "macos") {
            ("open", vec![path])
        } else {
            ("xdg-open", vec![path])
        };

        if let Err(e) = std::process::Command::new(executable).args(&args).spawn() {
            log::error!(
                "failed to open {target:?} from UI: {e:?} (command line: \"{executable} {args:?})"
            );
            return Err(e);
        }
    }
    Ok(())
}

pub fn backup_exe_path() -> Option<PathBuf> {
    match std::env::current_exe() {
        Ok(pb) => match pb.file_name() {
            None => None,
            Some(file_name) => {
                if let Some(file_name_str) = file_name.to_str() {
                    Some(pb.with_file_name(format!("{file_name_str}.old")))
                } else {
                    log::warn!("Executable filename contains invalid UTF-8: {file_name:?}");
                    None
                }
            }
        },
        Err(e) => {
            log::warn!("Failed to read current executable path: {e:?}");
            None
        }
    }
}

pub fn cleanup_after_update() {
    if let Some(old_backup) = backup_exe_path() {
        if !old_backup.exists() {
            return;
        }

        let start_time = std::time::Instant::now();
        let max_duration = std::time::Duration::from_secs(180);
        let mut sleep_duration = std::time::Duration::from_millis(100);

        let mut last_error = None;
        while start_time.elapsed() < max_duration {
            match std::fs::remove_file(&old_backup) {
                Err(e) => {
                    last_error = Some(e);
                    std::thread::sleep(sleep_duration);
                    sleep_duration = std::cmp::min(
                        sleep_duration.mul_f32(1.5),
                        std::time::Duration::from_secs(5),
                    );
                }
                Ok(_) => {
                    log::debug!(
                        "removed old executable ({:?}) which was left after SUCCESSFUL update. time spent waiting: {:?}",
                        old_backup, start_time.elapsed()
                    );
                    return;
                }
            }
        }

        if let Some(e) = last_error {
            log::warn!(
                "failed to remove old executable ({old_backup:?}) which was left after SUCCESSFUL update after {max_duration:?}: {e:?}"
            );
        }
    }
}

pub fn restart(executable_path: Option<PathBuf>) -> std::io::Result<()> {
    let exe_path = executable_path
        .or(std::env::current_exe().ok())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine executable path for restart",
            )
        })?;

    log::debug!("restart: going to launch another copy of myself and then exit");
    std::process::Command::new(&exe_path)
        .arg(&exe_path)
        .spawn()?;
    std::process::exit(0);
}

pub fn fix_cwd() -> Result<(), std::io::Error> {
    let image = std::env::current_exe()?;
    if let Some(desired_dir) = image.parent() {
        if std::env::current_dir()? != desired_dir {
            std::env::set_current_dir(desired_dir)?;
        }
    }
    Ok(())
}
