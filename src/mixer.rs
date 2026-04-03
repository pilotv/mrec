/// Mix two f32 PCM streams sample-by-sample. Shorter stream is zero-padded.
/// Output is clamped to [-1.0, 1.0].
pub fn mix_streams(a: &[f32], b: &[f32]) -> Vec<f32> {
    let len = a.len().max(b.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let sa = a.get(i).copied().unwrap_or(0.0);
        let sb = b.get(i).copied().unwrap_or(0.0);
        out.push((sa + sb).clamp(-1.0, 1.0));
    }
    out
}

/// Linear interpolation resample for interleaved stereo audio.
/// Resamples each channel independently to avoid cross-channel interpolation artifacts.
/// For same rate, returns a clone.
pub fn resample(input: &[f32], from_rate: u32, to_rate: u32, channels: u16) -> Vec<f32> {
    if from_rate == to_rate || input.is_empty() {
        return input.to_vec();
    }

    let ch = channels as usize;
    let num_frames = input.len() / ch;
    let ratio = from_rate as f64 / to_rate as f64;
    let out_frames = ((num_frames as f64) / ratio).round() as usize;
    let mut output = Vec::with_capacity(out_frames * ch);

    for frame in 0..out_frames {
        let src_pos = frame as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;

        let idx0 = idx.min(num_frames - 1);
        let idx1 = (idx + 1).min(num_frames - 1);

        for c in 0..ch {
            let s0 = input[idx0 * ch + c];
            let s1 = input[idx1 * ch + c];
            output.push(s0 + frac * (s1 - s0));
        }
    }

    output
}
