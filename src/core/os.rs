pub fn open_in_file_explorer(target: &str) -> std::io::Result<()> {
    let target = match target.starts_with(".") {
        false => std::path::Path::new(target).to_path_buf(),
        true => std::env::current_exe()?.parent().unwrap().join(target),
    };

    log::debug!("requested to open {:?}", target);
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
        }
    }
    Ok(())
}

pub fn cleanup_after_update() {
    if let Ok(executable) = std::env::current_exe() {
        let mut old_backup = executable.clone();
        old_backup.set_file_name(format!(
            "{}.old",
            executable.file_name().unwrap().to_str().unwrap()
        ));

        if !old_backup.exists() {
            return;
        }

        let max_retries = 5;
        let mut attempts = 0;

        while attempts < max_retries {
            match std::fs::remove_file(&old_backup) {
                Ok(_) => {
                    log::debug!("Removed old executable successfully");
                    return;
                }
                Err(e) => {
                    if (cfg!(windows) || e.kind() == std::io::ErrorKind::PermissionDenied)
                        && attempts < max_retries - 1
                    {
                        log::warn!(
                            "Failed to remove old executable (attempt {}/{}): {:?}",
                            attempts + 1,
                            max_retries,
                            e
                        );
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(
                            500 * (attempts as u64),
                        ));
                    } else {
                        log::error!("Permanent failure to remove old executable: {:?}", e);
                        return;
                    }
                }
            }
        }
    }
}

pub fn restart() {
    if let Ok(mut image) = std::env::current_exe() {
        let old_suffix = ".old";
        if image.to_string_lossy().ends_with(old_suffix) {
            image.set_file_name(
                image
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .trim_end_matches(old_suffix),
            );
        }

        log::debug!("restart: going to launch new copy at {:?} and exit", image);
        if let Err(e) = std::process::Command::new(&image).spawn() {
            log::error!("failed to restart myself: {:?}", e);
        } else {
            std::process::exit(0);
        }
    }
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
