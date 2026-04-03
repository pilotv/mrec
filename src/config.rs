use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AudioSource {
    SystemOnly,
    MicrophoneOnly,
    Both,
}

impl Default for AudioSource {
    fn default() -> Self {
        AudioSource::Both
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: PathBuf,
    pub bitrate: u32,
    pub audio_source: AudioSource,
    pub microphone: Option<String>,
    pub filename_template: String,
}

impl Default for Config {
    fn default() -> Self {
        let output_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("recordings");

        Self {
            output_dir,
            bitrate: 192,
            audio_source: AudioSource::Both,
            microphone: None,
            filename_template: "mrec_{date}_{time}".to_string(),
        }
    }
}

impl Config {
    pub fn load_from(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(path).map_err(|e| format!("read config: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("parse config: {e}"))
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        fs::write(path, json).map_err(|e| format!("write config: {e}"))
    }

    pub fn default_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join("mrec.json")))
            .unwrap_or_else(|| PathBuf::from("mrec.json"))
    }

    pub fn format_filename(&self) -> String {
        let now = chrono::Local::now();
        let name = self
            .filename_template
            .replace("{date}", &now.format("%Y-%m-%d").to_string())
            .replace("{time}", &now.format("%H-%M-%S").to_string());
        format!("{name}.mp3")
    }

    pub fn valid_bitrates() -> &'static [u32] {
        &[128, 192, 256, 320]
    }
}
