//! How the guard reads a process on macOS, through libproc: the one it
//! knows best is itself.

use disk_guard::macos::processes;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::{Command, Stdio};
use std::ptr;

#[test]
fn this_process_reads_as_itself() {
    let mut mapped = tempfile::NamedTempFile::new().unwrap();
    mapped.write_all(&[0; 4096]).unwrap();
    // SAFETY: a fresh read-only mapping of a file this test holds open,
    // unmapped below and never read through.
    let region = unsafe {
        libc::mmap(
            ptr::null_mut(),
            4096,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            mapped.as_file().as_raw_fd(),
            0,
        )
    };
    assert_ne!(region, libc::MAP_FAILED);
    let all = processes(Path::new("/proc")).unwrap();
    // SAFETY: region is the 4096 bytes mapped above.
    unsafe { libc::munmap(region, 4096) };
    let me = all
        .iter()
        .find(|p| p.pid == std::process::id())
        .expect("not among the processes");
    assert!(me.readable);
    // Resolved, as libproc gives it, since TMPDIR lies under a link.
    let held_at = fs::canonicalize(mapped.path()).unwrap();
    assert!(me.files.contains(&held_at), "{:?}", me.files);
    assert!(me.maps.contains(&held_at), "{:?}", me.maps);
    let program = fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
    assert!(me.maps.contains(&program), "{:?}", me.maps);
}

// macOS gives a process's environment only to root, which the store's
// pass runs as: on macOS 26 a user's `KERN_PROCARGS2` of its own child
// ends after the arguments. So a test, run as a user, proves the
// arguments, and the environment is proven on Linux alone.
#[test]
fn a_process_holds_the_store_paths_its_command_line_names() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    // A writer handed its payload by path, with no file of it open: a
    // shell waiting on its input, its arguments naming two store paths.
    // Spawning returns once it has started, so its arguments are
    // already its own.
    let mut writer = Command::new("/bin/sh")
        .args(["-c", "read -r _"])
        .arg(format!("--payload={}/fff-payload/boot", store.display()))
        .arg(store.join("ggg-image.raw"))
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let held = disk_guard::garbage::runtime_roots(Path::new("/proc"), &store);
    // Its input closing ends it.
    drop(writer.stdin.take());
    writer.wait().unwrap();
    let mut roots: Vec<String> = held.paths.into_iter().collect();
    roots.sort_unstable();
    let named = |name: &str| store.join(name).to_string_lossy().into_owned();
    assert_eq!(
        roots,
        [named("fff-payload"), named("ggg-image.raw")],
        "the writer's arguments weren't read as holding them"
    );
}
