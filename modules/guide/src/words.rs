//! Secrets a person keeps on their phone and types back from it: ordinary
//! words, which a phone's camera reads off the screen and a person types
//! from the phone without a slip. They come from the EFF's large list
//! (`words.txt`, as published at
//! <https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt>, SHA-256
//! addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e).

use std::sync::OnceLock;

use crate::{Fault, Secret};

/// How many words a kept secret has: about 90 bits, out of reach of anyone
/// guessing, since the disk makes every guess wait on its key derivation.
pub const COUNT: usize = 7;

/// The list's 7,776 words, each once.
pub fn list() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        include_str!("words.txt")
            .lines()
            .filter_map(|line| line.split('\t').nth(1))
            .collect()
    })
}

/// Seven words, each picked from the list by the system's randomness, one
/// space apart.
pub fn made() -> Result<Secret, Fault> {
    let words = list();
    // Only draws below the largest multiple of the list's length count, so
    // no word is likelier than another.
    let limit = (usize::from(u16::MAX) + 1) / words.len() * words.len();
    let mut picked = Vec::with_capacity(COUNT);
    while picked.len() < COUNT {
        let mut bytes = [0u8; 2];
        getrandom::fill(&mut bytes)
            .map_err(|why| Fault::unreadable("the system's randomness", why))?;
        let drawn = usize::from(u16::from_le_bytes(bytes));
        if drawn < limit {
            picked.push(words[drawn % words.len()]);
        }
    }
    Ok(Secret::new(picked.join(" ")))
}

/// The stick's passphrase in 1Password: the stick's own, whichever
/// machine it was written for, as its owner named it.
pub const STICK_PHRASE: &str = "NixOS Install Phrase";

/// A machine's own secret in 1Password, as its owner names them: "Hedgehog
/// Recovery Phrase" for `kind` "Recovery" on the machine "hedgehog".
pub fn machines(machine: &str, kind: &str) -> String {
    let mut letters = machine.chars();
    let named: String = letters
        .next()
        .map(|first| first.to_uppercase().chain(letters).collect())
        .unwrap_or_default();
    format!("{named} {kind} Phrase")
}

/// How a person keeps a secret on their iPhone, in 1Password, under
/// `name`, said the same way for every secret: the steps before the words
/// are shown alone, for `Ask::Show`, and what they do once it's kept.
pub fn keeping(what: &str, name: &str, why: &str) -> String {
    format!(
        "{why}, so keep it in 1Password on your phone. Return shows its seven words alone on the screen. Then:\n\
         1. On your iPhone, open the Camera and point it at the words.\n\
         2. Tap the Live Text button when it shows in the corner, then tap Copy All.\n\
         3. Open 1Password, tap +, then Password.\n\
         4. Name it \"{name}\", paste the words as its password, and tap Save. If an earlier install left one by that name, open it instead, tap Edit, paste the words over its password, and tap Save.\n\
         5. Press Return here, and type {what} back, reading it off your phone."
    )
}

/// What was typed, as the words it says: lower case, one space apart, so a
/// capital or a doubled space isn't a different secret. A recovery key
/// systemd made, in lower-case groups with no spaces, is left as it is.
pub fn typed(text: &str) -> String {
    text.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    #[test]
    fn the_list_is_the_effs_with_every_word_once() {
        let digest = Sha256::digest(include_bytes!("words.txt"));
        let hex = digest.iter().fold(String::new(), |mut hex, byte| {
            // Writing to a String can't fail.
            _ = write!(hex, "{byte:02x}");
            hex
        });
        assert_eq!(
            hex,
            "addd35536511597a02fa0a9ff1e5284677b8883b83e986e43f15a3db996b903e"
        );
        let words = list();
        assert_eq!(words.len(), 7776);
        let unique: std::collections::BTreeSet<_> = words.iter().collect();
        assert_eq!(unique.len(), words.len());
        assert!(words.iter().all(|word| !word.contains(char::is_whitespace)));
    }

    #[test]
    fn a_secret_is_seven_words_from_the_list_drawn_afresh() {
        let made = made().unwrap();
        let words: Vec<&str> = made.expose().split(' ').collect();
        assert_eq!(words.len(), COUNT);
        assert!(words.iter().all(|word| list().contains(word)), "{words:?}");
        let words = f64::from(u32::try_from(list().len()).unwrap());
        let count = f64::from(u32::try_from(COUNT).unwrap());
        assert!(words.log2() * count > 90.0);
        assert_ne!(made.expose(), super::made().unwrap().expose());
    }

    #[test]
    fn a_machines_secret_is_named_for_it_as_its_owner_names_them() {
        assert_eq!(machines("hedgehog", "Recovery"), "Hedgehog Recovery Phrase");
        assert_eq!(machines("laptop", "Rescue"), "Laptop Rescue Phrase");
    }

    #[test]
    fn a_reinstall_says_to_write_over_the_item_it_already_kept() {
        let how = keeping(
            "the recovery key",
            "a disk recovery key",
            "It opens the disk",
        );
        assert!(how.contains("Name it \"a disk recovery key\""), "{how}");
        assert!(
            how.contains("If an earlier install left one by that name"),
            "{how}"
        );
    }

    #[test]
    fn typed_words_are_read_as_the_words_they_say() {
        assert_eq!(typed("  Abacus   yo-yo\tzoom \n"), "abacus yo-yo zoom");
        let systemds = "fcbdhjkl-cedrbtnj-gkvvbclr-dlvdejbh-kgccgijv-rhjujdrn-ivlecdcr-tkbrclgt";
        assert_eq!(typed(systemds), systemds);
    }
}
