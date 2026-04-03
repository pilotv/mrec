use mp3lame_encoder::{Builder, Encoder, FlushNoGap, InterleavedPcm};
use std::io::Write;

pub struct Mp3Encoder<W: Write> {
    encoder: Encoder,
    writer: W,
    mp3_buffer: Vec<u8>,
}

impl<W: Write> Mp3Encoder<W> {
    /// Create a new MP3 encoder. bitrate_kbps: 128, 192, 256, or 320.
    pub fn new(sample_rate: u32, channels: u32, bitrate_kbps: u32, writer: W) -> Result<Self, String> {
        let mut builder = Builder::new().ok_or("Failed to create LAME builder")?;
        builder
            .set_sample_rate(sample_rate)
            .map_err(|e| format!("set_sample_rate: {e:?}"))?;
        builder
            .set_num_channels(channels as u8)
            .map_err(|e| format!("set_num_channels: {e:?}"))?;

        let brate = match bitrate_kbps {
            128 => mp3lame_encoder::Birtate::Kbps128,
            192 => mp3lame_encoder::Birtate::Kbps192,
            256 => mp3lame_encoder::Birtate::Kbps256,
            320 => mp3lame_encoder::Birtate::Kbps320,
            _ => mp3lame_encoder::Birtate::Kbps192,
        };
        builder
            .set_brate(brate)
            .map_err(|e| format!("set_brate: {e:?}"))?;
        builder
            .set_quality(mp3lame_encoder::Quality::Best)
            .map_err(|e| format!("set_quality: {e:?}"))?;

        let encoder = builder.build().map_err(|e| format!("build encoder: {e:?}"))?;
        let mp3_buffer = vec![0u8; 16384];

        Ok(Self {
            encoder,
            writer,
            mp3_buffer,
        })
    }

    /// Encode interleaved f32 PCM samples [-1.0, 1.0]
    pub fn encode(&mut self, samples: &[f32]) -> Result<(), String> {
        let pcm_i16: Vec<i16> = samples
            .iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();

        let input = InterleavedPcm(&pcm_i16);

        let needed = (1.25 * pcm_i16.len() as f64) as usize + 7200;
        if self.mp3_buffer.len() < needed {
            self.mp3_buffer.resize(needed, 0);
        }

        let encoded_size = self
            .encoder
            .encode(input, &mut self.mp3_buffer)
            .map_err(|e| format!("encode: {e:?}"))?;

        if encoded_size > 0 {
            self.writer
                .write_all(&self.mp3_buffer[..encoded_size])
                .map_err(|e| format!("write: {e}"))?;
        }

        Ok(())
    }

    /// Flush remaining MP3 data
    pub fn flush(&mut self) -> Result<(), String> {
        let flushed_size = self
            .encoder
            .flush::<FlushNoGap>(&mut self.mp3_buffer)
            .map_err(|e| format!("flush: {e:?}"))?;

        if flushed_size > 0 {
            self.writer
                .write_all(&self.mp3_buffer[..flushed_size])
                .map_err(|e| format!("write: {e}"))?;
        }

        self.writer.flush().map_err(|e| format!("flush writer: {e}"))?;
        Ok(())
    }
}
