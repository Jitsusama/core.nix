use super::half;

#[test]
fn halves_read_as_singles() {
    for (bits, want, within) in [
        (0x3c00, 1.0, f32::EPSILON),
        (0xc000, -2.0, f32::EPSILON),
        (0x3555, 0.333_251_95, 1e-7),
        (0x0000, 0.0, f32::EPSILON),
        (0x0001, 5.960_464_5e-8, 1e-12),
    ] {
        let got = half(bits);
        assert!(
            (got - want).abs() < within,
            "{bits:#06x} came to {got}, not {want}"
        );
    }
    let infinity = half(0x7c00);
    assert!(
        infinity.is_infinite(),
        "0x7c00 came to {infinity}, not infinity"
    );
}
