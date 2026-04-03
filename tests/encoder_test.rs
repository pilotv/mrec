#[test]
fn test_encode_silence_produces_mp3_output() {
    let mut output = Vec::new();
    let mut encoder = mrec::encoder::Mp3Encoder::new(44100, 2, 192, &mut output).unwrap();

    let silence = vec![0.0f32; 44100 * 2];
    encoder.encode(&silence).unwrap();
    encoder.flush().unwrap();

    assert!(!output.is_empty(), "MP3 output should not be empty");
    assert_eq!(output[0], 0xFF, "First byte should be MP3 sync");
    assert!(output[1] & 0xE0 == 0xE0, "Second byte should have sync bits");
}

#[test]
fn test_encode_sine_wave() {
    let mut output = Vec::new();
    let sample_rate = 44100u32;
    let mut encoder = mrec::encoder::Mp3Encoder::new(sample_rate, 2, 192, &mut output).unwrap();

    let samples: Vec<f32> = (0..sample_rate as usize)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            [sample, sample]
        })
        .collect();

    encoder.encode(&samples).unwrap();
    encoder.flush().unwrap();

    assert!(output.len() > 100, "Sine wave MP3 should have substantial data");
}

#[test]
fn test_encode_different_bitrates() {
    let sample_rate = 44100u32;
    let samples: Vec<f32> = (0..sample_rate as usize)
        .flat_map(|i| {
            let t = i as f32 / sample_rate as f32;
            let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            [s, s]
        })
        .collect();

    let mut out_128 = Vec::new();
    let mut enc = mrec::encoder::Mp3Encoder::new(sample_rate, 2, 128, &mut out_128).unwrap();
    enc.encode(&samples).unwrap();
    enc.flush().unwrap();

    let mut out_320 = Vec::new();
    let mut enc = mrec::encoder::Mp3Encoder::new(sample_rate, 2, 320, &mut out_320).unwrap();
    enc.encode(&samples).unwrap();
    enc.flush().unwrap();

    assert!(out_320.len() > out_128.len(), "320kbps ({}) should be larger than 128kbps ({})", out_320.len(), out_128.len());
}
