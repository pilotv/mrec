#[test]
fn test_mix_equal_length_streams() {
    let stream_a = vec![0.5f32, -0.5, 0.3, -0.3];
    let stream_b = vec![0.2f32, -0.2, 0.1, -0.1];
    let mixed = mrec::mixer::mix_streams(&stream_a, &stream_b);

    assert_eq!(mixed.len(), 4);
    let expected: Vec<f32> = stream_a
        .iter()
        .zip(stream_b.iter())
        .map(|(a, b)| (a + b).clamp(-1.0, 1.0))
        .collect();
    for (got, exp) in mixed.iter().zip(expected.iter()) {
        assert!((got - exp).abs() < 1e-6, "got {got}, expected {exp}");
    }
}

#[test]
fn test_mix_different_length_pads_shorter() {
    let stream_a = vec![0.5f32, 0.5, 0.5, 0.5];
    let stream_b = vec![0.3f32, 0.3];
    let mixed = mrec::mixer::mix_streams(&stream_a, &stream_b);

    assert_eq!(mixed.len(), 4);
    assert!((mixed[0] - 0.8).abs() < 1e-6);
    assert!((mixed[2] - 0.5).abs() < 1e-6);
}

#[test]
fn test_mix_clamps_output() {
    let stream_a = vec![0.9f32];
    let stream_b = vec![0.9f32];
    let mixed = mrec::mixer::mix_streams(&stream_a, &stream_b);

    assert_eq!(mixed.len(), 1);
    assert!((mixed[0] - 1.0).abs() < 1e-6, "should clamp to 1.0");
}

#[test]
fn test_resample_double() {
    let input = vec![0.0f32, 1.0, 0.0, -1.0];
    let output = mrec::mixer::resample(&input, 22050, 44100);

    assert_eq!(output.len(), 8);
    assert!((output[0] - 0.0).abs() < 1e-6);
}

#[test]
fn test_resample_same_rate_is_identity() {
    let input = vec![0.1f32, 0.2, 0.3, 0.4];
    let output = mrec::mixer::resample(&input, 44100, 44100);
    assert_eq!(output.len(), input.len());
    for (a, b) in output.iter().zip(input.iter()) {
        assert!((a - b).abs() < 1e-6);
    }
}
