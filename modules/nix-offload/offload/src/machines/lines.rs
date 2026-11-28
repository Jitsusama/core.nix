//! The machines lines the files say: a lent builder's line for its big
//! work or for any work, the standing builders read from their file, and
//! a file's text from its lines.

/// The work a lent builder is for: a derivation needing one of these is
/// sent to it first. Anything else stays here, since copying its inputs
/// out and its output back takes longer than building it: on 29 October a
/// system build that takes 34 s to 2.5 min here ran past ten minutes once
/// its small derivations went out. VM tests stay too: their machines are
/// built here, and copying those out is most of their time over a slow
/// upload. On 29 October one took 17 minutes there, its test 55 s of it,
/// against 60 s here. Sent to the builder as a command naming a commit
/// instead, they're fetched, evaluated and built there, and only the
/// command crosses.
const BIG: [&str; 1] = ["big-parallel"];

/// What a VM test needs, kept out of what the lent line offers, so neither
/// Nix nor the machine queue, which counts a builder's VM room by them,
/// takes it for a place to run one.
const VM: [&str; 2] = ["kvm", "nixos-test"];

/// A lent builder's machines line written once for each kind of big work
/// it supports, with that kind mandatory, so Nix sends it only that work,
/// and offering nothing a VM test needs. What the line already requires
/// stays required beside that kind, since a builder taking only some
/// builds, such as one whose x86 runs through Rosetta, would otherwise be
/// sent every build of that kind. Nix counts a builder's slots by its URI,
/// so the lines share one set. A line supporting none of them is kept as
/// it is.
pub fn for_big_work(line: &str) -> String {
    let (fields, offered) = offering(line);
    let supported: Vec<&str> = fields[5].split(',').collect();
    let required: Vec<&str> = fields[6]
        .split(',')
        .filter(|feature| !feature.is_empty() && *feature != "-")
        .collect();
    let lines: Vec<String> = BIG
        .iter()
        .filter(|kind| supported.contains(kind))
        .map(|kind| {
            let mut mandatory = required.clone();
            if !mandatory.contains(kind) {
                mandatory.push(kind);
            }
            let mandatory = mandatory.join(",");
            let mut fields = fields.clone();
            fields[5] = &offered;
            fields[6] = &mandatory;
            fields.join(" ")
        })
        .collect();
    if lines.is_empty() {
        line.to_owned()
    } else {
        lines.join("\n")
    }
}

/// A lent builder's machines line with nothing mandatory, so Nix sends it
/// any piece that asks for nothing it lacks, still offering nothing a VM
/// test needs.
pub fn for_any_work(line: &str) -> String {
    let (mut fields, offered) = offering(line);
    fields[5] = &offered;
    fields[6] = "-";
    fields.join(" ")
}

/// A line's eight fields, and the features it supports less a VM test's.
fn offering(line: &str) -> (Vec<&str>, String) {
    let mut fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 8 {
        fields.resize(8, "-");
    }
    let offered = fields[5]
        .split(',')
        .filter(|feature| !VM.contains(feature))
        .collect::<Vec<_>>()
        .join(",");
    (fields, offered)
}

/// A builder always there to lend, such as another machine at home, named
/// by the address that says whether it answers and its Nix machines line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub address: String,
    pub line: String,
}

/// The standing builders in a file of them, one a line, its address and
/// then its machines line: `192.168.4.46:22 ssh-ng://builder@home ...`.
/// Blank lines and those starting `#` say nothing.
pub fn standing(text: &str) -> Vec<Standing> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (address, line) = line.split_once(char::is_whitespace)?;
            Some(Standing {
                address: address.to_owned(),
                line: line.trim().to_owned(),
            })
        })
        .collect()
}

/// The file's text, as it should read now.
pub fn text<'a>(answering: impl Iterator<Item = &'a str>, lent: Option<&'a str>) -> String {
    answering.chain(lent).fold(String::new(), |mut text, line| {
        text.push_str(line);
        text.push('\n');
        text
    })
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
