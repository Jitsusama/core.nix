//! A released command's output as the agent is given it: the value, and
//! the forms a command most often prints it in (base64, URL escaping and
//! JSON escaping), each replaced, and the whole cut at a megabyte. A
//! command that means to show the value can always show it some other
//! way, so this keeps it out of a session by accident, not by force; the
//! one yes and the one fixed checker are what keep it from an agent here.

/// What stands in for the value wherever it was printed.
pub const MASK: &str = "[released value]";
/// The most output an answer carries.
pub const MOST_BYTES: usize = 1 << 20;

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The value in standard base64, padded.
fn base64(value: &str) -> String {
    let mut out = String::new();
    for chunk in value.as_bytes().chunks(3) {
        let bytes = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let joined = u32::from(bytes[0]) << 16 | u32::from(bytes[1]) << 8 | u32::from(bytes[2]);
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(char::from(BASE64[(joined >> (18 - 6 * at) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The value with every byte but URL's unreserved ones escaped.
fn url_escaped(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// The value as a JSON string holds it, without its quotes.
fn json_escaped(value: &str) -> String {
    let quoted = serde_json::to_string(value).unwrap_or_default();
    quoted
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_default()
        .to_owned()
}

/// The output with the value in each of its forms masked, and whether it
/// was cut at [`MOST_BYTES`].
pub fn masked(output: &str, value: &str) -> (String, bool) {
    let mut forms = vec![
        value.to_owned(),
        base64(value),
        url_escaped(value),
        json_escaped(value),
    ];
    // The base64 of a value printed inside a longer text may lose its
    // padding, so its unpadded form is masked too.
    forms.push(base64(value).trim_end_matches('=').to_owned());
    // Longest first, so a form holding another is masked whole.
    forms.sort_by_key(|form| std::cmp::Reverse(form.len()));
    forms.dedup();
    let mut said = output.to_owned();
    for form in forms.iter().filter(|form| !form.is_empty()) {
        said = said.replace(form.as_str(), MASK);
    }
    if said.len() <= MOST_BYTES {
        return (said, false);
    }
    let mut end = MOST_BYTES;
    while !said.is_char_boundary(end) {
        end -= 1;
    }
    said.truncate(end);
    (said, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_form_of_the_value_is_masked() {
        let value = "s3cret value/+";
        let printed = format!(
            "plain {value}; b64 {}; url {}; json {}",
            base64(value),
            url_escaped(value),
            json_escaped(value)
        );
        let (said, cut) = masked(&printed, value);
        assert!(!cut, "a short output was cut");
        assert!(!said.contains("s3cret"), "the value showed: {said}");
        assert!(!said.contains(&base64(value)), "its base64 showed: {said}");
        assert!(!said.contains("s3cret%20"), "its URL form showed: {said}");
        assert_eq!(
            said.matches(MASK).count(),
            4,
            "not every form was masked: {said}"
        );
    }

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64("Man"), "TWFu", "three bytes encoded wrong");
        assert_eq!(base64("Ma"), "TWE=", "two bytes encoded wrong");
        assert_eq!(base64("M"), "TQ==", "one byte encoded wrong");
    }

    #[test]
    fn a_long_output_is_cut_at_a_megabyte() {
        let (said, cut) = masked(&"é".repeat(MOST_BYTES), "x");
        assert!(cut, "a long output wasn't said to be cut");
        assert!(said.len() <= MOST_BYTES, "the output passed a megabyte");
    }
}
