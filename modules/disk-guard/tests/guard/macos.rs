//! How the guard reads a process on macOS, through libproc: the one it
//! knows best is itself.

use disk_guard::macos::processes;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::Path;
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
