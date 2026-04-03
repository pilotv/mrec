#![windows_subsystem = "windows"]

pub mod capture;
pub mod config;
pub mod encoder;
pub mod mixer;
pub mod recorder;
pub mod settings_ui;

use config::Config;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use recorder::Recorder;
use tray_icon::{Icon, TrayIconBuilder};

fn main() {
    // Load config
    let config_path = Config::default_path();
    let mut config = Config::load_from(&config_path).unwrap_or_default();

    // Build tray menu
    let menu = Menu::new();
    let item_start = MenuItem::new("Start Recording", true, None);
    let item_stop = MenuItem::new("Stop Recording", false, None);
    let item_settings = MenuItem::new("Settings...", true, None);
    let item_exit = MenuItem::new("Exit", true, None);

    menu.append(&item_start).unwrap();
    menu.append(&item_stop).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_settings).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&item_exit).unwrap();

    let id_start = item_start.id().clone();
    let id_stop = item_stop.id().clone();
    let id_settings = item_settings.id().clone();
    let id_exit = item_exit.id().clone();

    let icon_idle = create_icon([128, 128, 128, 255]);
    let icon_recording = create_icon([220, 40, 40, 255]);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("mrec - idle")
        .with_icon(icon_idle.clone())
        .build()
        .expect("Failed to create tray icon");

    let mut active_recorder: Option<Recorder> = None;
    let menu_rx = MenuEvent::receiver();

    loop {
        // Pump Win32 messages
        unsafe {
            let mut msg: windows_sys::Win32::UI::WindowsAndMessaging::MSG = std::mem::zeroed();
            while windows_sys::Win32::UI::WindowsAndMessaging::PeekMessageW(
                &mut msg, 0, 0, 0,
                windows_sys::Win32::UI::WindowsAndMessaging::PM_REMOVE,
            ) != 0
            {
                windows_sys::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
                windows_sys::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
            }
        }

        if let Ok(event) = menu_rx.try_recv() {
            if event.id() == &id_start {
                match Recorder::start(config.clone()) {
                    Ok(rec) => {
                        active_recorder = Some(rec);
                        item_start.set_enabled(false);
                        item_stop.set_enabled(true);
                        item_settings.set_enabled(false);
                        tray.set_icon(Some(icon_recording.clone())).ok();
                        tray.set_tooltip(Some("mrec - RECORDING")).ok();
                    }
                    Err(e) => {
                        show_error(&format!("Failed to start recording:\n{e}"));
                    }
                }
            } else if event.id() == &id_stop {
                if let Some(mut rec) = active_recorder.take() {
                    match rec.stop() {
                        Ok(path) => {
                            tray.set_tooltip(Some(&format!("mrec - saved: {}", path.file_name().unwrap_or_default().to_string_lossy()))).ok();
                        }
                        Err(e) => {
                            show_error(&format!("Error stopping recording:\n{e}"));
                        }
                    }
                }
                item_start.set_enabled(true);
                item_stop.set_enabled(false);
                item_settings.set_enabled(true);
                tray.set_icon(Some(icon_idle.clone())).ok();
            } else if event.id() == &id_settings {
                if let Some(new_config) = settings_ui::show_settings(&config) {
                    config = new_config;
                    let _ = config.save_to(&config_path);
                }
            } else if event.id() == &id_exit {
                if let Some(mut rec) = active_recorder.take() {
                    let _ = rec.stop();
                }
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Create a 16x16 solid color circle icon
fn create_icon(color: [u8; 4]) -> Icon {
    let size = 16u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - 7.5;
            let dy = y as f32 - 7.5;
            if dx * dx + dy * dy <= 7.0 * 7.0 {
                rgba.extend_from_slice(&color);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Icon::from_rgba(rgba, size, size).expect("Failed to create icon")
}

fn show_error(msg: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let wide_msg: Vec<u16> = OsStr::new(msg).encode_wide().chain(Some(0)).collect();
    let wide_title: Vec<u16> = OsStr::new("mrec Error").encode_wide().chain(Some(0)).collect();

    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            0,
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}
