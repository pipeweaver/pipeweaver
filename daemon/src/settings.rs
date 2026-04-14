use crate::APP_NAME_ID;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use log::{info, warn};
use pipeweaver_ipc::commands::GlobalSettings;
use std::fs;
use std::fs::{File, create_dir_all};
use std::io::ErrorKind;
use std::path::PathBuf;

pub fn load_settings() -> GlobalSettings {
    match File::open(get_settings_file()) {
        Ok(reader) => {
            let settings = serde_json::from_reader(reader);
            settings.unwrap_or_else(|e| {
                warn!(
                    "[Settings] Found, but unable to Load ({}), sending default",
                    e
                );
                GlobalSettings::default()
            })
        }
        Err(_) => {
            warn!("[Settings] Not Found, sending default");
            GlobalSettings::default()
        }
    }
}

pub fn save_settings(settings: GlobalSettings) -> Result<()> {
    info!("[Settings] Saving");
    let file_path = get_settings_file();

    if let Some(parent) = file_path.parent()
        && let Err(e) = create_dir_all(parent)
        && e.kind() != ErrorKind::AlreadyExists
    {
        return Err(e).context(format!(
            "Could not create config directory at {}",
            parent.to_string_lossy()
        ))?;
    }

    if file_path.exists() {
        fs::remove_file(&file_path).context("Unable to remove old Profile")?;
    }

    let file = File::create(file_path)?;
    serde_json::to_writer_pretty(file, &settings)?;

    info!("[Settings] Saved");
    Ok(())
}

fn get_settings_file() -> PathBuf {
    // We'll never get here if the project dir can't be found, that'll bail in main
    let dirs =
        ProjectDirs::from("io", "github", APP_NAME_ID).expect("Failed to get project directory");

    let config_dir = dirs.config_dir().to_path_buf();
    config_dir.join(format!("{}-settings.json", APP_NAME_ID))
}
