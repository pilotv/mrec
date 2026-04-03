use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleRate, Stream, StreamConfig};
use crossbeam_channel::Sender;

pub struct CaptureConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Get the default output device (for WASAPI loopback — system audio)
pub fn loopback_device() -> Result<(Device, CaptureConfig), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device found")?;

    let config = device
        .default_output_config()
        .map_err(|e| format!("output config: {e}"))?;

    Ok((
        device,
        CaptureConfig {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
        },
    ))
}

/// List all available input devices (microphones) with their names
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Get a specific input device by name, or the default if name is None
pub fn microphone_device(name: Option<&str>) -> Result<(Device, CaptureConfig), String> {
    let host = cpal::default_host();

    let device = match name {
        Some(target_name) => {
            let devices = host.input_devices().map_err(|e| format!("list devices: {e}"))?;
            devices
                .into_iter()
                .find(|d| d.name().ok().as_deref() == Some(target_name))
                .ok_or_else(|| format!("Microphone '{target_name}' not found"))?
        }
        None => host
            .default_input_device()
            .ok_or("No input device found")?,
    };

    let config = device
        .default_input_config()
        .map_err(|e| format!("input config: {e}"))?;

    Ok((
        device,
        CaptureConfig {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
        },
    ))
}

/// Build an input stream that sends f32 samples to a channel
pub fn build_capture_stream(
    device: &Device,
    config: &CaptureConfig,
    sender: Sender<Vec<f32>>,
) -> Result<Stream, String> {
    let stream_config = StreamConfig {
        channels: config.channels,
        sample_rate: SampleRate(config.sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let err_fn = |err| eprintln!("Audio stream error: {err}");

    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let _ = sender.send(data.to_vec());
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("build stream: {e}"))?;

    stream.play().map_err(|e| format!("play stream: {e}"))?;

    Ok(stream)
}
