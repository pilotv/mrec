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

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
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

    // Volume levels from config (percent → linear gain)
    let system_vol = config.system_volume as f32 / 100.0;
    let mic_vol = config.mic_volume as f32 / 100.0;

    // Fixed chunk size: 20ms worth of stereo samples at output rate
    // This ensures consistent timing between loopback and mic streams
    let chunk_samples = (out_rate as usize / 50) * 2; // 20ms * 2 channels

    // Accumulation buffers for steady chunk-based processing
    let mut loopback_accum: Vec<f32> = Vec::with_capacity(chunk_samples * 2);
    let mut mic_accum: Vec<f32> = Vec::with_capacity(chunk_samples * 2);

    // Recording loop
    while !stop_flag.load(Ordering::Relaxed) {
        // Drain available loopback samples into accumulator
        loop {
            match loopback_rx.try_recv() {
                Ok(data) => {
                    let stereo = to_stereo(&data, loopback_channels);
                    loopback_accum.extend(stereo);
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        // Drain available mic samples into accumulator
        loop {
            match mic_rx.try_recv() {
                Ok(data) => {
                    let stereo = to_stereo(&data, mic_cfg_channels);
                    let resampled = if need_resample {
                        mixer::resample(&stereo, mic_rate, out_rate, 2)
                    } else {
                        stereo
                    };
                    mic_accum.extend(resampled);
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }

        // Process full chunks
        let has_loopback = _loopback_stream.is_some();
        let has_mic = _mic_stream.is_some();

        // Determine how many samples we can process this iteration
        let available = if has_loopback && has_mic {
            // Both sources: process the minimum available (keeps them in sync)
            loopback_accum.len().min(mic_accum.len())
        } else if has_loopback {
            loopback_accum.len()
        } else {
            mic_accum.len()
        };

        // Only process full chunks (aligned to stereo frames)
        let process_len = (available / chunk_samples) * chunk_samples;

        if process_len == 0 {
            thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }

        let output = if has_loopback && has_mic {
            let lb: Vec<f32> = loopback_accum.drain(..process_len).collect();
            let mc: Vec<f32> = mic_accum.drain(..process_len).collect();
            mixer::mix_streams(&lb, &mc, system_vol, mic_vol)
        } else if has_loopback {
            loopback_accum.drain(..process_len).collect()
        } else {
            mic_accum.drain(..process_len).collect()
        };

        if !output.is_empty() {
            mp3.encode(&output)?;
        }
    }

    // Flush remaining accumulated samples
    let remaining = if _loopback_stream.is_some() && _mic_stream.is_some() {
        let len = loopback_accum.len().min(mic_accum.len());
        if len > 0 {
            let lb: Vec<f32> = loopback_accum.drain(..len).collect();
            let mc: Vec<f32> = mic_accum.drain(..len).collect();
            mixer::mix_streams(&lb, &mc, system_vol, mic_vol)
        } else if !loopback_accum.is_empty() {
            loopback_accum
        } else {
            mic_accum
        }
    } else if !loopback_accum.is_empty() {
        loopback_accum
    } else {
        mic_accum
    };

    if !remaining.is_empty() {
        mp3.encode(&remaining)?;
    }

    mp3.flush()?;
    Ok(file_path)
}

fn to_stereo(samples: &[f32], channels: u16) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    match channels {
        1 => samples.iter().flat_map(|&s| [s, s]).collect(),
        2 => samples.to_vec(),
        n => {
            // Downmix: take first two channels from each frame
            let n = n as usize;
            samples
                .chunks_exact(n)
                .flat_map(|frame| [frame[0], frame[1]])
                .collect()
        }
    }
}
