use crate::capture;
use crate::config::{AudioSource, Config};
use crate::encoder::Mp3Encoder;
use crate::mixer;
use crossbeam_channel::{self, TryRecvError};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct Recorder {
    stop_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<Result<PathBuf, String>>>,
}

impl Recorder {
    pub fn start(config: Config) -> Result<Self, String> {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        let thread_handle = thread::spawn(move || run_recording(stop_clone, config));

        Ok(Self {
            stop_flag,
            thread_handle: Some(thread_handle),
        })
    }

    pub fn stop(&mut self) -> Result<PathBuf, String> {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.thread_handle
            .take()
            .ok_or("Already stopped".to_string())?
            .join()
            .map_err(|_| "Recording thread panicked".to_string())?
    }

    pub fn is_recording(&self) -> bool {
        !self.stop_flag.load(Ordering::Relaxed)
    }
}

fn run_recording(stop_flag: Arc<AtomicBool>, config: Config) -> Result<PathBuf, String> {
    fs::create_dir_all(&config.output_dir).map_err(|e| format!("create dir: {e}"))?;

    let file_path = config.output_dir.join(config.format_filename());
    let file = File::create(&file_path).map_err(|e| format!("create file: {e}"))?;
    let writer = BufWriter::new(file);

    // Set up devices based on audio source config
    let loopback = match config.audio_source {
        AudioSource::MicrophoneOnly => None,
        _ => Some(capture::loopback_device()?),
    };

    let mic = match config.audio_source {
        AudioSource::SystemOnly => None,
        _ => Some(capture::microphone_device(config.microphone.as_deref())?),
    };

    // Determine output sample rate (prefer loopback, fallback to mic)
    let out_rate = loopback
        .as_ref()
        .map(|(_, c)| c.sample_rate)
        .or_else(|| mic.as_ref().map(|(_, c)| c.sample_rate))
        .ok_or("No audio device configured")?;

    let out_channels = 2u32;
    let mut mp3 = Mp3Encoder::new(out_rate, out_channels, config.bitrate, writer)?;

    // Set up capture channels
    let (loopback_tx, loopback_rx) = crossbeam_channel::bounded::<Vec<f32>>(64);
    let (mic_tx, mic_rx) = crossbeam_channel::bounded::<Vec<f32>>(64);

    // Start capture streams
    let _loopback_stream = match &loopback {
        Some((dev, cfg)) => Some(capture::build_capture_stream(dev, cfg, loopback_tx)?),
        None => {
            drop(loopback_tx);
            None
        }
    };

    let _mic_stream = match &mic {
        Some((dev, cfg)) => Some(capture::build_capture_stream(dev, cfg, mic_tx)?),
        None => {
            drop(mic_tx);
            None
        }
    };

    let loopback_channels = loopback.as_ref().map(|(_, c)| c.channels).unwrap_or(2);
    let mic_cfg_channels = mic.as_ref().map(|(_, c)| c.channels).unwrap_or(1);
    let mic_rate = mic.as_ref().map(|(_, c)| c.sample_rate).unwrap_or(out_rate);
    let need_resample = mic_rate != out_rate;

    // Recording loop
    while !stop_flag.load(Ordering::Relaxed) {
        let mut loopback_buf = Vec::new();
        loop {
            match loopback_rx.try_recv() {
                Ok(data) => loopback_buf.extend(data),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        let mut mic_buf = Vec::new();
        loop {
            match mic_rx.try_recv() {
                Ok(data) => mic_buf.extend(data),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        if loopback_buf.is_empty() && mic_buf.is_empty() {
            thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        // Convert to stereo
        let loopback_stereo = to_stereo(&loopback_buf, loopback_channels);
        let mic_stereo = to_stereo(&mic_buf, mic_cfg_channels);

        // Resample mic if needed
        let mic_final = if need_resample && !mic_stereo.is_empty() {
            mixer::resample(&mic_stereo, mic_rate, out_rate)
        } else {
            mic_stereo
        };

        // Mix or use single source
        let output = if !loopback_stereo.is_empty() && !mic_final.is_empty() {
            mixer::mix_streams(&loopback_stereo, &mic_final)
        } else if !loopback_stereo.is_empty() {
            loopback_stereo
        } else {
            mic_final
        };

        if !output.is_empty() {
            mp3.encode(&output)?;
        }
    }

    mp3.flush()?;
    Ok(file_path)
}

fn to_stereo(samples: &[f32], channels: u16) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    if channels == 1 {
        samples.iter().flat_map(|&s| [s, s]).collect()
    } else {
        samples.to_vec()
    }
}
