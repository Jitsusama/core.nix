//! A checkout's branch count and config size: each branch counted once
//! wherever git keeps it, and no count rather than zero where the
//! branches can't be read.

use super::*;

#[test]
fn a_checkout_counts_its_branches_once_each_and_its_configs_bytes() {
    let src = place("branches");
    let folder = src.join("heavy");
    checkout_at(&folder, Some("https://git.example.org/jg/heavy.git"), 100);
    let git = folder.join(".git");
    // main is loose already; jg/a is loose alone, jg/b packed alone, and
    // main packed too, as a branch written since the last pack is.
    write(&git.join("refs/heads/jg/a"), &format!("{COMMIT}\n"));
    write(&git.join("refs/heads/jg/half.lock"), &format!("{OTHER}\n"));
    // A dot-named file is no branch git would name, and a link is passed
    // over rather than followed, so neither is counted.
    write(&git.join("refs/heads/.stray"), &format!("{OTHER}\n"));
    std::os::unix::fs::symlink(git.join("refs/heads/jg/a"), git.join("refs/heads/jg/link"))
        .expect("a link");
    // The tag's peeled line opens with ^ and names no ref.
    write(
        &git.join("packed-refs"),
        &format!(
            "# pack-refs with: peeled fully-peeled sorted\n{COMMIT} refs/heads/jg/b\n\
             {COMMIT} refs/heads/main\n{OTHER} refs/remotes/origin/main\n\
             {OTHER} refs/tags/v1\n^{COMMIT}\n"
        ),
    );
    let config = fs::metadata(git.join("config")).expect("the config").len();
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(
        (checkout.branches, checkout.config_bytes),
        (Some(3), Some(config)),
        "{checkout:#?}"
    );
}

#[test]
fn a_checkout_whose_packed_refs_cant_be_read_says_no_count_rather_than_zero() {
    let src = place("unread");
    let folder = src.join("locked");
    checkout_at(&folder, Some("https://git.example.org/jg/locked.git"), 100);
    let git = folder.join(".git");
    // A packed-refs that is a folder can't be read as a file, as one with
    // its permissions taken away can't, and a test can't run as anyone but
    // root may.
    fs::create_dir_all(git.join("packed-refs")).expect("a folder in its place");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_whose_loose_branches_cant_be_read_says_no_count_rather_than_zero() {
    let src = place("unwalked");
    let folder = src.join("locked");
    checkout_at(&folder, Some("https://git.example.org/jg/locked.git"), 100);
    let git = folder.join(".git");
    // A refs/heads that is a file can't be walked as a folder.
    fs::remove_dir_all(git.join("refs/heads")).expect("its branches gone");
    write(&git.join("refs/heads"), "not a folder\n");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_whose_branches_folder_is_a_link_says_no_count_rather_than_walking_it() {
    let src = place("linked");
    let folder = src.join("linked");
    checkout_at(&folder, Some("https://git.example.org/jg/linked.git"), 100);
    let git = folder.join(".git");
    let elsewhere = src.join("elsewhere");
    write(&elsewhere.join("jg/a"), &format!("{COMMIT}\n"));
    fs::remove_dir_all(git.join("refs/heads")).expect("its branches gone");
    std::os::unix::fs::symlink(&elsewhere, git.join("refs/heads")).expect("a link");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_whose_reftable_cant_be_read_says_no_count_rather_than_zero() {
    let src = place("untabled");
    let folder = src.join("untabled");
    checkout_at(
        &folder,
        Some("https://git.example.org/jg/untabled.git"),
        100,
    );
    // A reftable folder with no tables.list is a stack that can't be read.
    fs::create_dir_all(folder.join(".git/reftable")).expect("a reftable folder");
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    assert_eq!(checkout.branches, None, "{checkout:#?}");
}

#[test]
fn a_checkout_never_packed_counts_its_loose_branches() {
    let src = place("unpacked");
    let folder = src.join("unpacked");
    checkout_at(
        &folder,
        Some("https://git.example.org/jg/unpacked.git"),
        100,
    );
    let git = folder.join(".git");
    write(&git.join("refs/heads/jg/a"), &format!("{COMMIT}\n"));
    assert!(
        !git.join("packed-refs").exists(),
        "the checkout has no packed-refs"
    );
    let record = reading(&[src], None);
    let checkout = &record.repositories[0].checkouts[0];
    // main and jg/a, with no packed-refs to add to them.
    assert_eq!(checkout.branches, Some(2), "{checkout:#?}");
}
