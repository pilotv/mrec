use crate::capture;
use crate::config::{AudioSource, Config};
use native_windows_gui as nwg;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Once;

static NWG_INIT: Once = Once::new();

/// Show a modal settings dialog. Returns updated Config if user clicks Save, None if Cancel.
pub fn show_settings(current: &Config) -> Option<Config> {
    NWG_INIT.call_once(|| { nwg::init().expect("Failed to init NWG"); });

    let result: Rc<RefCell<Option<Config>>> = Rc::new(RefCell::new(None));
    let config_clone = current.clone();

    // Window
    let mut window = nwg::Window::default();
    nwg::Window::builder()
        .size((420, 380))
        .position((300, 200))
        .title("mrec Settings")
        .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::VISIBLE)
        .build(&mut window)
        .expect("Failed to build window");

    // --- Output folder ---
    let mut lbl_folder = nwg::Label::default();
    nwg::Label::builder()
        .text("Output folder:")
        .position((10, 15))
        .size((100, 20))
        .parent(&window)
        .build(&mut lbl_folder)
        .unwrap();

    let mut txt_folder = nwg::TextInput::default();
    nwg::TextInput::builder()
        .text(&config_clone.output_dir.to_string_lossy())
        .position((110, 12))
        .size((220, 24))
        .parent(&window)
        .build(&mut txt_folder)
        .unwrap();

    let mut btn_browse = nwg::Button::default();
    nwg::Button::builder()
        .text("Browse...")
        .position((335, 11))
        .size((75, 26))
        .parent(&window)
        .build(&mut btn_browse)
        .unwrap();

    // --- Bitrate ---
    let mut lbl_bitrate = nwg::Label::default();
    nwg::Label::builder()
        .text("MP3 Quality:")
        .position((10, 55))
        .size((100, 20))
        .parent(&window)
        .build(&mut lbl_bitrate)
        .unwrap();

    let mut cmb_bitrate = nwg::ComboBox::default();
    nwg::ComboBox::builder()
        .position((110, 52))
        .size((150, 200))
        .collection(vec![
            "128 kbps".to_string(),
            "192 kbps".to_string(),
            "256 kbps".to_string(),
            "320 kbps".to_string(),
        ])
        .parent(&window)
        .build(&mut cmb_bitrate)
        .unwrap();

    let bitrate_idx = match config_clone.bitrate {
        128 => 0,
        192 => 1,
        256 => 2,
        320 => 3,
        _ => 1,
    };
    cmb_bitrate.set_selection(Some(bitrate_idx));

    // --- Audio source ---
    let mut lbl_source = nwg::Label::default();
    nwg::Label::builder()
        .text("Audio source:")
        .position((10, 95))
        .size((100, 20))
        .parent(&window)
        .build(&mut lbl_source)
        .unwrap();

    let mut cmb_source = nwg::ComboBox::default();
    nwg::ComboBox::builder()
        .position((110, 92))
        .size((200, 200))
        .collection(vec![
            "System + Microphone".to_string(),
            "System audio only".to_string(),
            "Microphone only".to_string(),
        ])
        .parent(&window)
        .build(&mut cmb_source)
        .unwrap();

    let source_idx = match config_clone.audio_source {
        AudioSource::Both => 0,
        AudioSource::SystemOnly => 1,
        AudioSource::MicrophoneOnly => 2,
    };
    cmb_source.set_selection(Some(source_idx));

    // --- Microphone selection ---
    let mut lbl_mic = nwg::Label::default();
    nwg::Label::builder()
        .text("Microphone:")
        .position((10, 135))
        .size((100, 20))
        .parent(&window)
        .build(&mut lbl_mic)
        .unwrap();

    let mic_list = capture::list_input_devices();
    let mut mic_items = vec!["(Default)".to_string()];
    mic_items.extend(mic_list.clone());

    let mut cmb_mic = nwg::ComboBox::default();
    nwg::ComboBox::builder()
        .position((110, 132))
        .size((300, 200))
        .collection(mic_items.clone())
        .parent(&window)
        .build(&mut cmb_mic)
        .unwrap();

    let mic_idx = match &config_clone.microphone {
        None => 0,
        Some(name) => mic_items.iter().position(|m| m == name).unwrap_or(0),
    };
    cmb_mic.set_selection(Some(mic_idx));

    // --- Filename template ---
    let mut lbl_fname = nwg::Label::default();
    nwg::Label::builder()
        .text("Filename:")
        .position((10, 175))
        .size((100, 20))
        .parent(&window)
        .build(&mut lbl_fname)
        .unwrap();

    let mut txt_fname = nwg::TextInput::default();
    nwg::TextInput::builder()
        .text(&config_clone.filename_template)
        .position((110, 172))
        .size((300, 24))
        .parent(&window)
        .build(&mut txt_fname)
        .unwrap();

    let mut lbl_hint = nwg::Label::default();
    nwg::Label::builder()
        .text("Placeholders: {date} = 2026-04-03, {time} = 15-30-00")
        .position((110, 200))
        .size((300, 20))
        .parent(&window)
        .build(&mut lbl_hint)
        .unwrap();

    // --- Buttons ---
    let mut btn_save = nwg::Button::default();
    nwg::Button::builder()
        .text("Save")
        .position((220, 330))
        .size((90, 32))
        .parent(&window)
        .build(&mut btn_save)
        .unwrap();

    let mut btn_cancel = nwg::Button::default();
    nwg::Button::builder()
        .text("Cancel")
        .position((320, 330))
        .size((90, 32))
        .parent(&window)
        .build(&mut btn_cancel)
        .unwrap();

    // Event handler
    let window_handle = window.handle;
    let result_clone = Rc::clone(&result);
    let handler = nwg::full_bind_event_handler(&window_handle, move |evt, _evt_data, handle| {
        match evt {
            nwg::Event::OnButtonClick => {
                if handle == btn_browse.handle {
                    let mut dialog = nwg::FileDialog::default();
                    nwg::FileDialog::builder()
                        .title("Select output folder")
                        .action(nwg::FileDialogAction::OpenDirectory)
                        .build(&mut dialog)
                        .unwrap();
                    if dialog.run(Some(&window)) {
                        if let Ok(path) = dialog.get_selected_item() {
                            txt_folder.set_text(&path.to_string_lossy());
                        }
                    }
                } else if handle == btn_save.handle {
                    let bitrate = match cmb_bitrate.selection() {
                        Some(0) => 128,
                        Some(2) => 256,
                        Some(3) => 320,
                        _ => 192,
                    };
                    let audio_source = match cmb_source.selection() {
                        Some(1) => AudioSource::SystemOnly,
                        Some(2) => AudioSource::MicrophoneOnly,
                        _ => AudioSource::Both,
                    };
                    let microphone = match cmb_mic.selection() {
                        Some(0) | None => None,
                        Some(i) => mic_items.get(i).cloned(),
                    };

                    let new_config = Config {
                        output_dir: PathBuf::from(txt_folder.text()),
                        bitrate,
                        audio_source,
                        microphone,
                        filename_template: txt_fname.text(),
                    };
                    *result_clone.borrow_mut() = Some(new_config);
                    nwg::stop_thread_dispatch();
                } else if handle == btn_cancel.handle {
                    nwg::stop_thread_dispatch();
                }
            }
            nwg::Event::OnWindowClose => {
                if handle == window_handle {
                    nwg::stop_thread_dispatch();
                }
            }
            _ => {}
        }
    });

    nwg::dispatch_thread_events();
    nwg::unbind_event_handler(&handler);

    Rc::try_unwrap(result)
        .ok()
        .and_then(|cell| cell.into_inner())
}
