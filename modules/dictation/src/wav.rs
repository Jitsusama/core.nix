//! 16 kHz mono 16-bit WAV files, the one kind Whisper's checks read.

/// The samples of a WAV file, as floats from -1 to 1.
///
/// # Errors
///
/// When it isn't a RIFF WAV of 16 kHz mono 16-bit PCM.
pub fn samples(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF WAV".to_owned());
    }
    let mut at = 12;
    let mut format = None;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = usize::try_from(u32::from_le_bytes([
            bytes[at + 4],
            bytes[at + 5],
            bytes[at + 6],
            bytes[at + 7],
        ]))
        .map_err(|_| "a chunk too big")?;
        let body = bytes
            .get(at + 8..at + 8 + size)
            .ok_or("a chunk runs past the end")?;
        match id {
            b"fmt " if body.len() >= 16 => {
                let word = |i: usize| u16::from_le_bytes([body[i], body[i + 1]]);
                let rate = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
                format = Some((word(0), word(2), rate, word(14)));
            }
            b"data" => {
                if format != Some((1, 1, 16_000, 16)) {
                    return Err(format!(
                        "wanted PCM, mono, 16 kHz, 16-bit; found {format:?}"
                    ));
                }
                return Ok(body
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&pair| f32::from(i16::from_le_bytes(pair)) / 32_768.0)
                    .collect());
            }
            _ => {}
        }
        at += 8 + size + size % 2;
    }
    Err("no data chunk".to_owned())
}

#[cfg(test)]
mod tests {
    use super::samples;

    fn wav(format: [u8; 16], data: &[u8]) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0".to_vec();
        bytes.extend_from_slice(&format);
        bytes.extend_from_slice(b"LIST\x01\0\0\0x\0");
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&u32::try_from(data.len()).expect("small").to_le_bytes());
        bytes.extend_from_slice(data);
        bytes
    }

    const MONO_16K: [u8; 16] = [1, 0, 1, 0, 0x80, 0x3e, 0, 0, 0, 0x7d, 0, 0, 2, 0, 16, 0];

    #[test]
    fn samples_read_past_other_chunks() {
        let got = samples(&wav(MONO_16K, &[0, 0x40, 0, 0xc0])).expect("it reads");
        assert_eq!(got, [0.5, -0.5]);
    }

    #[test]
    fn another_rate_is_refused() {
        let mut format = MONO_16K;
        format[4] = 0x44;
        format[5] = 0xac;
        assert!(samples(&wav(format, &[0, 0])).is_err());
    }

    #[test]
    fn something_else_is_refused() {
        assert!(samples(b"not a wav file at all").is_err());
    }
}
