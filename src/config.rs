use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Folder watched for .torrent files (auto-add).
    pub watch_folder: Option<String>,
    /// Download destination for new torrents.
    pub download_dir: String,
    /// Enable clipboard link pickup.
    pub clipboard_watch: bool,
    /// Default speed limit in MB/s applied to new torrents. None = unlimited.
    pub global_speed_limit: Option<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            watch_folder: None,
            download_dir: default_download_dir(),
            clipboard_watch: true,
            global_speed_limit: None,
        }
    }
}

fn default_download_dir() -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{home}/Downloads/tordln")
}

pub fn load() -> Settings {
    let path = settings_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(s: &Settings) -> anyhow::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let s = toml::to_string_pretty(s)?;
    std::fs::write(path, s)?;
    Ok(())
}

fn settings_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::Path::new(&home)
        .join(".config")
        .join("tordln")
        .join("settings.toml")
}
