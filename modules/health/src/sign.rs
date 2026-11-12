//! What a message says once what changes between its repeats is taken
//! out (numbers, ids, addresses, times), so the thousand times a program
//! says one thing are counted as one signature, and whether it's a stack
//! trace.

/// The longest a signature is kept, in characters; past it, the rest of a
/// message rarely tells two apart.
const LONGEST: usize = 160;

/// What a stack trace or a crash starts with, in the languages and
/// runtimes the machine runs: Rust, Python, QML and JavaScript, C's
/// signals and the kernel's own.
const TRACES: [&str; 9] = [
    "panicked at",
    "Traceback (most recent call last)",
    "TypeError:",
    "ReferenceError:",
    "segfault at",
    "SIGSEGV",
    "SIGABRT",
    "dumped core",
    "Call Trace:",
];

/// The message's first line with each word holding a digit made `#`, or
/// its digits alone when the rest of it is a name: `sda1` stays `sda#`,
/// while `0x7f3a`, `12:04:31` and an id become `#`.
#[must_use]
pub fn signature(message: &str) -> String {
    let first = message.lines().next().unwrap_or_default().trim();
    let mut signed = String::new();
    let mut word = String::new();
    for character in first.chars() {
        if character.is_ascii_alphanumeric() {
            word.push(character);
        } else {
            signed.push_str(&sign(&word));
            word.clear();
            signed.push(character);
        }
    }
    signed.push_str(&sign(&word));
    let collapsed = collapse(&signed);
    match collapsed.char_indices().nth(LONGEST) {
        Some((end, _)) => format!("{}…", &collapsed[..end]),
        None => collapsed,
    }
}

/// One word, signed: unchanged without a digit, `#` when it's a number in
/// any base, and otherwise its runs of digits made `#`.
fn sign(word: &str) -> String {
    if !word.chars().any(|c| c.is_ascii_digit()) {
        return word.to_owned();
    }
    if word.chars().all(|c| c.is_ascii_hexdigit() || c == 'x') {
        return "#".to_owned();
    }
    let mut signed = String::new();
    for character in word.chars() {
        if !character.is_ascii_digit() {
            signed.push(character);
        } else if !signed.ends_with('#') {
            signed.push('#');
        }
    }
    signed
}

/// Runs of `#` joined by what numbers are written with (`#.#`, `#:#:#`,
/// `#-#`) made one `#`, so a time or a version is one mark.
fn collapse(signed: &str) -> String {
    let mut out = String::new();
    let mut characters = signed.chars().peekable();
    while let Some(character) = characters.next() {
        let joins = matches!(character, '.' | ':' | '-' | ',');
        if joins && out.ends_with('#') && characters.peek() == Some(&'#') {
            characters.next();
            continue;
        }
        out.push(character);
    }
    out
}

/// Whether the message is a stack trace or a crash's.
#[must_use]
pub fn is_trace(message: &str) -> bool {
    TRACES.iter().any(|start| message.contains(start))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_changes_between_repeats_is_taken_out() {
        assert_eq!(
            signature("light 3 answered in 812 ms at 12:04:31.5"),
            "light # answered in # ms at #",
            "numbers and a time are one mark each"
        );
        assert_eq!(
            signature("Process 4169227 (chrome) of user 1000 dumped core."),
            "Process # (chrome) of user # dumped core.",
            "a pid and a user are marks"
        );
        assert_eq!(
            signature("nvme0n1: I/O error at 0x7f3a, dev sda1"),
            "nvme#n#: I/O error at #, dev sda#",
            "a device keeps its name, an address doesn't"
        );
    }

    #[test]
    fn a_messages_first_line_signs_it() {
        let trace = "Process 1 (chrome) of user 1000 dumped core.\n\n#0  0x64d8 n/a";
        assert_eq!(signature(trace), "Process # (chrome) of user # dumped core.", "first line");
    }

    #[test]
    fn a_long_message_is_cut() {
        let long = "word ".repeat(100);
        let signed = signature(&long);
        assert_eq!(signed.chars().count(), LONGEST + 1, "kept to its length and a mark");
        assert!(signed.ends_with('…'), "the cut says so: {signed}");
    }

    #[test]
    fn a_trace_is_known_in_each_language() {
        for message in [
            "thread 'main' panicked at src/main.rs:3:5",
            "Traceback (most recent call last):",
            "qrc:/Launcher.qml:12: TypeError: Cannot read property 'x' of null",
            "chrome[123]: segfault at 0 ip 0000 sp 0000 error 4",
        ] {
            assert!(is_trace(message), "{message} is a trace");
        }
        assert!(!is_trace("light 3 answered in 812 ms"), "a warning isn't a trace");
    }
}
