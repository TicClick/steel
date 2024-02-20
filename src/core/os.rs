#[derive(Debug)]
enum OpenTarget<'opener> {
    AppFile(&'opener str),
    AppDirectory,
}

fn open_app_path(target: OpenTarget) {
    if let Ok(mut path) = std::env::current_exe() {
        match target {
            OpenTarget::AppFile(file_name) => path.set_file_name(file_name),
            OpenTarget::AppDirectory => {
                path = path.parent().unwrap().to_path_buf();
            }
        }
        if let Some(path) = path.to_str() {
            let path = path.to_owned();
            let (executable, args) = if cfg!(target_os = "windows") {
                // let file_arg = format!("/select,{}", log_path);
                ("explorer.exe", vec![path])
            } else if cfg!(target_os = "macos") {
                ("open", vec![path])
            } else {
                ("xdg-open", vec![path])
            };
            if let Err(e) = std::process::Command::new(executable).args(&args).spawn() {
                log::error!("failed to open {target:?} from UI: {e:?} (command line: \"{executable} {args:?})");
            }
        }
    }
}

pub fn open_runtime_log() {
    open_app_path(OpenTarget::AppFile("runtime.log"))
}
pub fn open_settings_file() {
    open_app_path(OpenTarget::AppFile("settings.yaml"))
}
pub fn open_own_directory() {
    open_app_path(OpenTarget::AppDirectory)
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
        if let Err(e) = std::fs::remove_file(&old_backup) {
            log::warn!(
                "failed to remove old executable ({:?}) which was left after SUCCESSFUL update: {:?}",
                old_backup,
                e
            );
        } else {
            log::debug!(
                "removed old executable ({:?}) which was left after SUCCESSFUL update",
                old_backup
            );
        }
    }
}

pub fn restart() {
    if let Ok(image) = std::env::current_exe() {
        log::debug!("restart: going to launch another copy of myself and then exit");
        if let Err(e) = std::process::Command::new(&image).arg(&image).spawn() {
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
