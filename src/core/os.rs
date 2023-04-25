pub fn open_runtime_log() {
    if let Ok(mut path) = std::env::current_exe() {
        path.set_file_name("runtime.log");
        if let Some(log_path) = path.to_str() {
            let log_path = log_path.to_owned();
            let (executable, args) = if cfg!(windows) {
                // let file_arg = format!("/select,{}", log_path);
                ("explorer.exe", vec![log_path])
            } else if cfg!(macos) {
                ("Finder.app", vec![log_path])
            } else {
                ("open", vec![log_path])
            };
            if let Err(e) = std::process::Command::new(executable).args(&args).spawn() {
                log::error!("failed to show the log file from UI: {e:?} (command line: \"{executable} {args:?})");
            }
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
