use crate::discover::{dir, file};
use disk_guard::activity::Process;
use disk_guard::git::{busy, places, temporaries};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const ME: u32 = 1000;

/// A repository's git directory with what a fetch, a repack and a loose
/// write leave staged, beside what git keeps for good.
fn repository(root: &Path) -> PathBuf {
    let git = dir(&root.join("git"));
    dir(&git.join("objects/tmp_objdir-packfile-uri-x1Yz2W"));
    dir(&git.join("objects/incoming-Ab12Cd"));
    file(&git.join("objects/pack/tmp_pack_Qw3Er4"), "half a pack");
    file(&git.join("objects/pack/tmp_idx_Ty5Ui6"), "half an index");
    file(&git.join("objects/ab/tmp_obj_Op7As8"), "half an object");
    file(&git.join("objects/pack/pack-1234.pack"), "a pack");
    file(&git.join("objects/pack/pack-1234.idx"), "its index");
    file(&git.join("objects/ab/cdef0123"), "an object");
    dir(&git.join("objects/info"));
    file(&git.join("objects/info/tmp_notes"), "not git's");
    file(&git.join("objects/tmp_note"), "not git's either");
    git
}

fn git(pid: u32, exe: &str) -> Process {
    Process {
        pid,
        uid: ME,
        exe: Some(PathBuf::from(exe)),
        cmdline: vec![OsString::from(exe)],
        readable: true,
        ..Default::default()
    }
}

#[test]
fn what_git_stages_and_never_moved_into_place_is_a_temporary() {
    let root = tempfile::tempdir().unwrap();
    let git = repository(root.path());
    let mut found = temporaries(&git);
    found.sort();
    let mut expected = vec![
        git.join("objects/ab/tmp_obj_Op7As8"),
        git.join("objects/incoming-Ab12Cd"),
        git.join("objects/pack/tmp_idx_Ty5Ui6"),
        git.join("objects/pack/tmp_pack_Qw3Er4"),
        git.join("objects/tmp_objdir-packfile-uri-x1Yz2W"),
    ];
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn a_repository_is_worked_on_from_its_git_directory_and_each_worktree() {
    let root = tempfile::tempdir().unwrap();
    let git = repository(root.path());
    let tree = root.path().join("trees/one");
    file(
        &git.join("worktrees/one/gitdir"),
        &format!("{}\n", tree.join(".git").display()),
    );
    let mut found = places(&git);
    found.sort();
    let mut expected = vec![git.clone(), tree];
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn a_git_working_in_a_worktree_keeps_the_repository_s_temporaries() {
    // A fetch run from a worktree stages into the shared git directory,
    // and between one file and the next it may hold none of them open.
    let places = [
        PathBuf::from("/home/u/big/git"),
        PathBuf::from("/home/u/big/trees/one"),
    ];
    let fetch = Process {
        cwd: Some(PathBuf::from("/home/u/big/trees/one/areas/core")),
        ..git(7, "/nix/store/abc-git/bin/git")
    };
    assert!(busy(&places, &[fetch], ME));
}

#[test]
fn a_remote_helper_reading_the_repository_keeps_its_temporaries() {
    let places = [PathBuf::from("/home/u/big/git")];
    let helper = Process {
        cwd: Some(PathBuf::from("/")),
        maps: vec![PathBuf::from("/home/u/big/git/objects/pack/pack-1234.idx")],
        ..git(8, "/nix/store/abc-git/libexec/git-core/git-remote-https")
    };
    assert!(busy(&places, &[helper], ME));
    let writer = Process {
        cwd: Some(PathBuf::from("/")),
        files: vec![PathBuf::from(
            "/home/u/big/git/objects/pack/tmp_pack_Qw3Er4",
        )],
        ..git(9, "/nix/store/abc-git/libexec/git-core/git-index-pack")
    };
    assert!(busy(&places, &[writer], ME));
}

#[test]
fn nothing_but_git_elsewhere_leaves_a_repository_unworked() {
    // An editor in the tree stages nothing, and a git in another
    // repository stages into that one.
    let places = [
        PathBuf::from("/home/u/big/git"),
        PathBuf::from("/home/u/big/trees/one"),
    ];
    let editor = Process {
        cwd: Some(PathBuf::from("/home/u/big/trees/one")),
        ..git(10, "/usr/bin/nvim")
    };
    let elsewhere = Process {
        cwd: Some(PathBuf::from("/home/u/src/core")),
        ..git(11, "/nix/store/abc-git/bin/git")
    };
    assert!(!busy(&places, &[editor, elsewhere], ME));
}

#[test]
fn a_program_named_for_git_and_git_s_own_daemons_leave_a_repository_unworked() {
    // Both run for hours, so counting them would keep the temporaries
    // for good: a tool's engine sat in a tree for 95 minutes, and git's
    // credential cache held a file of the repository it was started from.
    let places = [
        PathBuf::from("/home/u/big/git"),
        PathBuf::from("/home/u/big/trees/root"),
    ];
    let engine = Process {
        cwd: Some(PathBuf::from("/home/u/big/trees/root/areas")),
        ..git(13, "/nix/store/abc-git-engine/bin/git-engine")
    };
    let credentials = Process {
        cwd: Some(PathBuf::from("/home/u/.cache/git/credential")),
        files: vec![PathBuf::from("/home/u/big/git/reftable/tables.list")],
        cmdline: vec![
            OsString::from("/nix/store/abc-git/libexec/git-core/git"),
            OsString::from("credential-cache--daemon"),
            OsString::from("/home/u/.cache/git/credential/socket"),
        ],
        ..git(14, "/nix/store/abc-git/libexec/git-core/git")
    };
    let fsmonitor = Process {
        cwd: Some(PathBuf::from("/home/u/big/trees/root")),
        cmdline: vec![
            OsString::from("git"),
            OsString::from("-C"),
            OsString::from("/home/u/big/trees/root"),
            OsString::from("fsmonitor--daemon"),
            OsString::from("run"),
        ],
        ..git(15, "/nix/store/abc-git/bin/git")
    };
    assert!(!busy(&places, &[engine, credentials, fsmonitor], ME));
}

#[test]
fn an_account_s_own_git_that_cant_be_read_keeps_them_and_another_s_doesn_t() {
    let places = [PathBuf::from("/home/u/big/git")];
    // What can't be read is known only by the name it was called by.
    let hidden = Process {
        readable: false,
        exe: None,
        ..git(12, "git")
    };
    assert!(busy(&places, std::slice::from_ref(&hidden), ME));
    let root = Process { uid: 0, ..hidden };
    assert!(!busy(&places, &[root], ME));
}
