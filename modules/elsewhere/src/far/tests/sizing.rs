//! The far line sizing what it brings back: a pipe a job's process puts
//! where it reads a size, at a file find listed or at a copy's own name,
//! is missing and never opened, so it never holds the far line, and the
//! rest comes back.

use super::{Left, WAITS, far_line, handover, kind};
use crate::far::{Missed, bring_back};
use std::io::Read;
use std::path::Path;

/// Shell that puts a pipe at `$at`, also named `held` in the far run's
/// home, with a writer in the background that answers each open of it,
/// noting it in `opened` before the reader can read its end, so an open
/// never waits and is always seen. It ends at the first open once
/// `let-go` stands, which is how `let_go` ends it.
const PIPED: &str = "mkfifo \"$at\" && ln \"$at\" \"$HOME/held\" || exit 1\n\
     while { [ ! -e \"$HOME/let-go\" ] || exit; echo >> \"$HOME/opened\"; } > \"$HOME/held\"\n\
     do :; done < /dev/null > /dev/null 2>&1 &";

/// The job whose three files come back.
const JOB: &str = "mkdir out && printf a > out/a && printf b > out/b && printf c > out/c";

/// Has the far line bring back `JOB`'s three files, `program` putting a
/// pipe at out/b's file or copy as it does, and asks that nothing opened
/// the pipe and that the other two came back.
fn brings_the_rest_past_a_pipe(program: (&str, &str)) {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], JOB, &kind, None, &[program]);
    // Seen before the near side lands anything, which may read a pipe.
    let opened = home.join("opened").exists();
    let into = home.join("near");
    let brought = bring_back(&handover(&home), &into, &kind.brings_back, WAITS);
    let read = |name: &str| std::fs::read_to_string(into.join(name)).ok();
    let landed = ["out/a", "out/b", "out/c"].map(read);
    let_go(&home);
    assert!(
        !opened,
        "the far line opened a pipe put where it reads a size ({code:?}): {err}"
    );
    assert!(
        code == Some(0) && err.contains("elsewhere: out/b didn't come back"),
        "a pipe wasn't said as missing ({code:?}): {err}"
    );
    assert_eq!(
        brought,
        Err(Missed::Lost(
            "out/b didn't come back from the far side; the other 2 landed".to_owned()
        )),
        "the rest didn't come back past the pipe"
    );
    assert_eq!(
        landed,
        [Some("a".to_owned()), None, Some("c".to_owned())],
        "what came back came back wrong"
    );
}

/// Ends `PIPED`'s writer in `home`, waiting on its end, and clears `home`.
fn let_go(home: &Path) {
    let _ = std::fs::write(home.join("let-go"), "");
    // The writer waits on an open of its own, so this one doesn't wait,
    // and the read ends as the writer does.
    if let Ok(mut held) = std::fs::File::open(home.join("held")) {
        let _ = held.read_to_end(&mut Vec::new());
    }
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn a_pipe_put_at_a_file_once_it_is_listed_is_missing_and_never_opened() {
    // A find that, once it listed out's files, puts a pipe at out/b, as a
    // job's process may meanwhile.
    let find = format!(
        "PATH=${{PATH#*:}} find \"$@\" || exit\n\
         [ \"$1 $2 $3\" = 'out -type f' ] || exit 0\n\
         at=$PWD/out/b && rm \"$at\" || exit 1\n{PIPED}"
    );
    brings_the_rest_past_a_pipe(("find", find.as_str()));
}

#[test]
fn a_pipe_put_at_a_copy_s_name_once_it_crossed_is_missing_and_never_opened() {
    // A cp that, once out/b crossed, puts a pipe at its copy's name, as a
    // job's process may meanwhile.
    let cp = format!(
        "PATH=${{PATH#*:}} cp \"$@\" || exit\n\
         for at; do :; done\n\
         case \" $* \" in *' out/b '*) rm \"$at\" || exit 1 ;; *) exit 0 ;; esac\n{PIPED}"
    );
    brings_the_rest_past_a_pipe(("cp", cp.as_str()));
}
