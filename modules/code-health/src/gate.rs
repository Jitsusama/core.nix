//! `code-health stale TREE WAITING`, the stale measure held at a change:
//! what stale.rs finds in the tree, held against the waiting list as
//! tree.rs holds it, said and refused on.

use std::fs;
use std::path::Path;

use terminal_kit::{Context, Exit, Failure};

use crate::{stale, tree};

/// What the gate says of the files whose comments couldn't be read, each a
/// line, none where there are none.
fn unread_said(unread: &[&String]) -> Vec<String> {
    if unread.is_empty() {
        return Vec::new();
    }
    let mut said = vec![
        "These files' comments couldn't be read, so what they name can't be held; \
         what waits in them stands, and code-health's reader (stale.rs) must learn them. \
         Until it does, the waiting list may let one through as `FILE unread`:"
            .to_owned(),
    ];
    said.extend(unread.iter().map(|file| format!("  {file}")));
    said
}

/// Each comment in `tree` naming a path that's gone, but those `waiting`
/// lists, said and failed on, as is each line of `waiting` listed more
/// often than it's found.
pub fn stale_held(context: &Context, tree: &str, waiting: &str) -> Result<(), Failure> {
    let (paths, files) = tree::files(Path::new(tree))
        .map_err(|what| Failure::new(Exit::NotFound, "the tree couldn't be read").because(what))?;
    let listed = fs::read_to_string(waiting).map_err(|why| {
        Failure::new(Exit::NotFound, "the waiting list couldn't be read")
            .because(format!("{waiting}: {why}"))
    })?;
    let listed: Vec<String> = listed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let Some(refused) = stale_verdict(&stale::stale(&paths, &files), &listed) else {
        return Ok(());
    };
    context.say(&refused).map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })?;
    Err(Failure::new(
        Exit::Refused,
        "a comment or the waiting list is stale, or a file's comments couldn't be read",
    )
    .because("each is named above"))
}

/// What refuses the change in `stale` held against `listed`, its waiting
/// list, if anything does. A file whose comments couldn't be read refuses
/// it too, though it's this tool's misreading: a gate passing it would let
/// a stale comment in it through unseen. The list may let one through by
/// name, since a repository built on core can't change core's reader, and
/// a waiver is told to come off once its file reads. What the list holds
/// of such a file stands, so it isn't taken for fixed while it can't be
/// read.
fn stale_verdict(stale: &stale::Stale, listed: &[String]) -> Option<String> {
    let (unread, spent) = tree::waived(&stale.unread, listed);
    let (new, mut gone) = tree::held(&stale.gone, &stale.unread, listed);
    gone.extend(spent);
    let mut said = unread_said(&unread);
    if !new.is_empty() {
        said.push("These comments name what isn't there; name where it is now:".to_owned());
        said.extend(new.iter().map(|named| format!("  {named}")));
    }
    if !gone.is_empty() {
        said.push("These wait on less now; list each as often as it's found:".to_owned());
        said.extend(gone.iter().map(|line| format!("  {line}")));
    }
    // Said as lines, each ended, so the refusal after them starts its own.
    (!said.is_empty()).then(|| said.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first line the gate says of the files it couldn't read.
    const UNREAD: &str = "These files' comments couldn't be read, so what they name can't be \
                          held; what waits in them stands, and code-health's reader \
                          (stale.rs) must learn them. Until it does, the waiting list may \
                          let one through as `FILE unread`:";

    fn owned(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|&line| line.to_owned()).collect()
    }

    #[test]
    fn the_gate_names_each_file_it_couldnt_read_or_says_nothing() {
        assert_eq!(
            (
                unread_said(&[]),
                unread_said(&[&"a.sh".to_owned(), &"b/c.py".to_owned()])
            ),
            (
                Vec::new(),
                vec![
                    UNREAD.to_owned(),
                    "  a.sh".to_owned(),
                    "  b/c.py".to_owned()
                ]
            ),
            "the files whose comments couldn't be read weren't each named, or were said of none"
        );
    }

    #[test]
    fn an_unread_file_refuses_the_change_while_its_entries_stand() {
        // A file named with a space, so its entries are told by their
        // last space, as a path holds none.
        let stale = stale::Stale {
            gone: owned(&["a.sh:3: old/x", "d.sh:1: new/y"]),
            unread: owned(&["b c.sh"]),
        };
        let listed = owned(&["a.sh old/x", "b c.sh old/z", "b c.sh old/z", "e.sh gone/w"]);
        assert_eq!(
            stale_verdict(&stale, &listed),
            Some(format!(
                "{UNREAD}\n  b c.sh\n\
                 These comments name what isn't there; name where it is now:\n  \
                 d.sh:1: new/y\n\
                 These wait on less now; list each as often as it's found:\n  \
                 e.sh gone/w (found nowhere: take it off)\n"
            )),
            "an unread file wasn't named first, its entries didn't stand, or a new or gone \
             one wasn't refused"
        );
        let unread_only = stale::Stale {
            gone: Vec::new(),
            unread: owned(&["b c.sh"]),
        };
        let every_read = stale::Stale {
            gone: owned(&["b c.sh:1: old/z"]),
            unread: Vec::new(),
        };
        assert_eq!(
            (
                stale_verdict(&unread_only, &owned(&["b c.sh old/z"])),
                stale_verdict(&every_read, &owned(&["b c.sh old/z"]))
            ),
            (Some(format!("{UNREAD}\n  b c.sh\n")), None),
            "a file that couldn't be read alone didn't refuse the change, or one read and \
             listed did"
        );
    }

    #[test]
    fn a_waived_unread_file_passes_till_it_reads_and_its_waiver_goes_then() {
        let unread = |files: &[&str]| stale::Stale {
            gone: Vec::new(),
            unread: owned(files),
        };
        assert_eq!(
            stale_verdict(
                &unread(&["b c.sh"]),
                &owned(&["b c.sh unread", "b c.sh old/z"])
            ),
            None,
            "a waived unread file refused the change, or its entries didn't stand"
        );
        assert_eq!(
            stale_verdict(
                &unread(&["b c.sh", "f.py"]),
                &owned(&["b c.sh unread", "g.py unread", "f.py unread", "f.py unread"])
            ),
            Some(
                "These wait on less now; list each as often as it's found:\n  \
                 f.py unread (listed 2 times: list it once)\n  \
                 g.py unread (read now or gone: take it off)\n"
                    .to_owned()
            ),
            "a waiver listed twice, or one whose file reads now, wasn't told to come off"
        );
    }

    #[test]
    fn a_waiver_lets_through_only_its_own_file() {
        let unread = stale::Stale {
            gone: Vec::new(),
            unread: owned(&["b c.sh", "f.py"]),
        };
        assert_eq!(
            stale_verdict(&unread, &owned(&["f.py unread", "b c.sh old/z"])),
            Some(format!("{UNREAD}\n  b c.sh\n")),
            "a waiver let through a file it doesn't name, or didn't let its own through"
        );
    }
}
