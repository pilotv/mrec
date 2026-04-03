#[test]
fn test_default_config() {
    let cfg = mrec::config::Config::default();
    assert_eq!(cfg.bitrate, 192);
    assert_eq!(cfg.audio_source, mrec::config::AudioSource::Both);
    assert_eq!(cfg.filename_template, "mrec_{date}_{time}");
    assert!(cfg.output_dir.to_string_lossy().contains("recordings"));
    assert_eq!(cfg.microphone, None);
}

#[test]
fn test_config_roundtrip_json() {
    let mut cfg = mrec::config::Config::default();
    cfg.bitrate = 320;
    cfg.audio_source = mrec::config::AudioSource::SystemOnly;
    cfg.microphone = Some("USB Mic".to_string());
    cfg.filename_template = "meeting_{date}".to_string();

    let json = serde_json::to_string_pretty(&cfg).unwrap();
    let loaded: mrec::config::Config = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.bitrate, 320);
    assert_eq!(loaded.audio_source, mrec::config::AudioSource::SystemOnly);
    assert_eq!(loaded.microphone, Some("USB Mic".to_string()));
    assert_eq!(loaded.filename_template, "meeting_{date}");
}

#[test]
fn test_config_save_load_file() {
    let dir = std::env::temp_dir().join("mrec_test_config");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("mrec.json");

    let mut cfg = mrec::config::Config::default();
    cfg.bitrate = 256;
    cfg.save_to(&path).unwrap();

    let loaded = mrec::config::Config::load_from(&path).unwrap();
    assert_eq!(loaded.bitrate, 256);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_config_load_missing_file_returns_default() {
    let path = std::path::PathBuf::from("/tmp/mrec_nonexistent_12345.json");
    let cfg = mrec::config::Config::load_from(&path).unwrap();
    assert_eq!(cfg.bitrate, 192);
}

#[test]
fn test_format_filename() {
    let cfg = mrec::config::Config {
        filename_template: "rec_{date}_{time}".to_string(),
        ..Default::default()
    };
    let name = cfg.format_filename();
    assert!(name.contains('-'), "filename should contain date separators");
    assert!(name.starts_with("rec_"), "should start with template prefix");
    assert!(name.ends_with(".mp3"), "should end with .mp3");
}
